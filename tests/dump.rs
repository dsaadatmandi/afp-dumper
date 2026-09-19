use std::fs;
use std::path::PathBuf;
use std::process::Command;

// builds AFP fixtures byte by byte so the tests do not depend on sample files
mod afp {
    pub const BPF: [u8; 3] = [0xd3, 0xa8, 0xa5];
    pub const EPF: [u8; 3] = [0xd3, 0xa9, 0xa5];
    pub const BDT: [u8; 3] = [0xd3, 0xa8, 0xa8];
    pub const EDT: [u8; 3] = [0xd3, 0xa9, 0xa8];
    pub const BNG: [u8; 3] = [0xd3, 0xa8, 0xad];
    pub const ENG: [u8; 3] = [0xd3, 0xa9, 0xad];
    pub const BPG: [u8; 3] = [0xd3, 0xa8, 0xaf];
    pub const EPG: [u8; 3] = [0xd3, 0xa9, 0xaf];
    pub const NOP: [u8; 3] = [0xd3, 0xee, 0xee];
    pub const TLE: [u8; 3] = [0xd3, 0xa0, 0x90];
    pub const IPD: [u8; 3] = [0xd3, 0xee, 0xfb]; // image data — type X'EE' like NOP

    // X'5A' framing, then SFLength (counting itself), type id, flags, reserved
    pub fn sf(type_id: [u8; 3], data: &[u8]) -> Vec<u8> {
        let mut v = vec![0x5a];
        v.extend_from_slice(&((8 + data.len()) as u16).to_be_bytes());
        v.extend_from_slice(&type_id);
        v.extend_from_slice(&[0x00, 0x00, 0x00]);
        v.extend_from_slice(data);
        v
    }

    pub fn triplet(id: u8, body: &[u8]) -> Vec<u8> {
        let mut v = vec![(body.len() + 2) as u8, id];
        v.extend_from_slice(body);
        v
    }

    // enough of cp500 for the fixtures: letters, digits, space and dash
    pub fn ebcdic(s: &str) -> Vec<u8> {
        s.bytes()
            .map(|c| match c {
                b'A'..=b'I' => 0xc1 + (c - b'A'),
                b'J'..=b'R' => 0xd1 + (c - b'J'),
                b'S'..=b'Z' => 0xe2 + (c - b'S'),
                b'a'..=b'i' => 0x81 + (c - b'a'),
                b'j'..=b'r' => 0x91 + (c - b'j'),
                b's'..=b'z' => 0xa2 + (c - b's'),
                b'0'..=b'9' => 0xf0 + (c - b'0'),
                b' ' => 0x40,
                b'-' => 0x60,
                other => panic!("no cp500 mapping in fixture helper for {other:?}"),
            })
            .collect()
    }

    pub fn name8(s: &str) -> Vec<u8> {
        ebcdic(&format!("{s:<8}"))[..8].to_vec()
    }

    // TLE with attribute name and value, optionally a qualifier. the leading
    // X'01' declares a code page, which the dumper deliberately ignores
    pub fn tle(name: &str, value: &str, seq: Option<u32>) -> Vec<u8> {
        let mut data = triplet(0x01, &[0xff, 0xff, 0x01, 0xf4]);

        let mut fqn = vec![0x0b, 0x00];
        fqn.extend(ebcdic(name));
        data.extend(triplet(0x02, &fqn));

        let mut val = vec![0x00, 0x00];
        val.extend(ebcdic(value));
        data.extend(triplet(0x36, &val));

        if let Some(seq) = seq {
            let mut q = seq.to_be_bytes().to_vec();
            q.extend(1u32.to_be_bytes());
            data.extend(triplet(0x80, &q));
        }

        sf(TLE, &data)
    }
}

use afp::*;

// two documents: the first with two page groups, the second with none, so
// both position shapes appear
fn nested_fixture() -> Vec<u8> {
    [
        sf(BPF, &name8("PRINTF")),
        sf(NOP, &ebcdic("preamble note")),
        //
        sf(BDT, &[name8("DOC0"), vec![0x00, 0x00]].concat()),
        tle("DOCLEVEL", "top", None),
        sf(BNG, &name8("GRP0")),
        tle("ACCOUNT", "0012345", Some(7)),
        sf(BPG, &name8("PG0")),
        tle("PAGETAG", "p-zero", None),
        sf(NOP, b"Plain ASCII NOP 42"),
        sf(EPG, &[]),
        sf(BPG, &name8("PG1")),
        tle("PAGETAG", "p-one", None),
        sf(EPG, &[]),
        sf(ENG, &[]),
        sf(BNG, &name8("GRP1")),
        tle("ACCOUNT", "9999999", None),
        sf(BPG, &name8("PG2")),
        sf(EPG, &[]),
        sf(ENG, &[]),
        sf(EDT, &[]),
        //
        sf(BDT, &[name8("DOC1"), vec![0x00, 0x00]].concat()),
        sf(BPG, &name8("PG0")),
        tle("SECOND", "doc-one", None),
        sf(EPG, &[]),
        sf(EDT, &[]),
        sf(EPF, &[]),
    ]
    .concat()
}

struct Run {
    stdout: String,
    stderr: String,
}

fn run(fixture: &[u8], name: &str, args: &[&str]) -> Run {
    let dir = std::env::temp_dir().join(format!("afp_dumper_it_{name}"));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("input.afp");
    fs::write(&path, fixture).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_reader"))
        .arg("-i")
        .arg(&path)
        .args(args)
        .output()
        .expect("run afp-dumper");

    Run {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn dump_lines(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|l| l.contains(": NOP") || l.contains(": TLE"))
        .collect()
}

#[test]
fn reports_position_for_every_nop_and_tle() {
    let out = run(&nested_fixture(), "positions", &["--dump-all"]);
    let lines = dump_lines(&out.stdout);

    let positions: Vec<_> = lines.iter().map(|l| l.split(':').next().unwrap()).collect();
    assert_eq!(
        positions,
        vec![
            "preamble",
            "doc 0",
            "doc 0 group 0",
            "doc 0 group 0 page 0",
            "doc 0 group 0 page 0",
            "doc 0 group 0 page 1",
            "doc 0 group 1",
            // second document has no page groups, so the group is omitted and
            // the page index restarts
            "doc 1 page 0",
        ]
    );
}

#[test]
fn decodes_tle_attribute_name_and_value_from_triplets() {
    let out = run(&nested_fixture(), "tle", &["--dump-tle"]);
    let lines = dump_lines(&out.stdout);

    assert_eq!(lines[1], "doc 0 group 0: TLE ACCOUNT = 0012345");
    assert!(lines.iter().any(|l| l.contains("SECOND = doc-one")));

    // nothing describing the field is printed: not the declared code page
    // (X'01'), not the attribute qualifier (X'80'), not the offset
    for marker in ["cpgid", "ccsid", "X'01'", "X'80'", "seq=", "lev=", "@"] {
        assert!(!out.stdout.contains(marker), "{marker} in {}", out.stdout);
    }
}

#[test]
fn picks_the_encoding_per_record() {
    let out = run(&nested_fixture(), "encoding", &["--dump-nop"]);
    let lines = dump_lines(&out.stdout);

    // same file, one ebcdic body and one ascii body, both decoded correctly
    assert_eq!(lines[0], "preamble: NOP preamble note");
    assert_eq!(lines[1], "doc 0 group 0 page 0: NOP Plain ASCII NOP 42");
}

#[test]
fn forcing_an_encoding_overrides_the_heuristic() {
    let out = run(
        &nested_fixture(),
        "forced",
        &["--dump-nop", "--encoding", "ebcdic"],
    );
    // the ebcdic body still reads, the ascii one no longer does — which is the
    // point of the override
    assert!(out.stdout.contains("preamble note"), "{}", out.stdout);
    assert!(!out.stdout.contains("Plain ASCII NOP 42"));
}

#[test]
fn ignores_matches_inside_payload_data() {
    // an image data record whose payload happens to contain the NOP type id
    let mut fixture = nested_fixture();
    fixture.extend(sf(
        IPD,
        b"\x00\x08\xd3\xee\xee\x00\x00\x00 image bytes",
    ));

    let out = run(&fixture, "falsepos", &["--dump-nop"]);
    assert_eq!(dump_lines(&out.stdout).len(), 2, "{}", out.stdout);
    assert!(out.stderr.contains("skipped 1 match"), "{}", out.stderr);
}

#[test]
fn indexing_runs_without_any_other_flag() {
    let out = run(&nested_fixture(), "index_only", &[]);
    assert!(
        out.stdout
            .contains("Found 2 document(s) containing 3 page group(s) across 4 page(s)"),
        "{}",
        out.stdout
    );
    assert!(dump_lines(&out.stdout).is_empty());
    assert!(!PathBuf::from("output").exists() || fs::read_dir("output").unwrap().count() == 0);
}

#[test]
fn terminal_dump_stops_above_the_limit() {
    let out = run(
        &nested_fixture(),
        "limit",
        &["--dump-all", "--dump-limit", "3"],
    );
    assert!(dump_lines(&out.stdout).is_empty(), "{}", out.stdout);
    assert!(out.stderr.contains("found 8 NOP/TLE record(s)"), "{}", out.stderr);
    assert!(out.stderr.contains("--dump-out"), "{}", out.stderr);
    // indexing still reports
    assert!(out.stdout.contains("Found 2 document(s)"));
}

#[test]
fn the_limit_does_not_apply_to_a_dump_file() {
    let target = std::env::temp_dir().join("afp_dumper_it_limitfile_tags.txt");
    let _ = fs::remove_file(&target);

    let out = run(
        &nested_fixture(),
        "limitfile",
        &[
            "--dump-all",
            "--dump-limit",
            "3",
            "--dump-out",
            target.to_str().unwrap(),
        ],
    );

    assert!(dump_lines(&out.stdout).is_empty());
    let written = fs::read_to_string(&target).unwrap();
    assert_eq!(dump_lines(&written).len(), 8, "{written}");
}
