use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Ascii,
    Ebcdic,
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Encoding::Ascii => write!(f, "ascii"),
            Encoding::Ebcdic => write!(f, "ebcdic"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingMode {
    Auto,
    Ascii,
    Ebcdic,
}

// how many significant bytes the sniffer looks at before committing
const PROBE_LEN: usize = 10;

// encoding is arbitrary per record and both can appear in one file, so it is
// decided per record from the bytes themselves: try ascii, and fall back to
// ebcdic as soon as one of the first PROBE_LEN significant bytes is not
// printable ascii. whatever code page a field declares is not consulted
//
// this works because ebcdic letters (C1-E9), digits (F0-F9) and its X'40' pad
// all sit at or above 0x80. testing "is alphanumeric" instead would misfire:
// under latin-1 those same ebcdic bytes decode to accented letters, which are
// alphanumeric, so real ebcdic text would be classified ascii
fn sniff(bytes: &[u8]) -> Encoding {
    let mut seen = 0;
    for &b in bytes {
        if matches!(b, b' ' | 0x00 | b'\t' | b'\r' | b'\n') {
            continue;
        }
        if !(0x20..=0x7e).contains(&b) {
            return Encoding::Ebcdic;
        }
        seen += 1;
        if seen == PROBE_LEN {
            break;
        }
    }
    Encoding::Ascii
}

pub fn decode(bytes: &[u8], mode: EncodingMode) -> (String, Encoding) {
    let encoding = match mode {
        EncodingMode::Auto => sniff(bytes),
        EncodingMode::Ascii => Encoding::Ascii,
        EncodingMode::Ebcdic => Encoding::Ebcdic,
    };

    let text = match encoding {
        Encoding::Ebcdic => bytes.iter().map(|&b| CP500[b as usize]).collect(),
        Encoding::Ascii => bytes.iter().map(|&b| b as char).collect(),
    };

    (text, encoding)
}

// newlines are handled by the caller, which splits them into separate output
// lines so every line keeps its position prefix
pub fn escape(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    for c in line.chars() {
        match c {
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

// cp500, ebcdic international. generated from the unicode mapping rather than
// hand-typed
#[rustfmt::skip]
const CP500: [char; 256] = [
    '\u{0000}', '\u{0001}', '\u{0002}', '\u{0003}', '\u{009C}', '\u{0009}', '\u{0086}', '\u{007F}', // 00
    '\u{0097}', '\u{008D}', '\u{008E}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{000E}', '\u{000F}', // 08
    '\u{0010}', '\u{0011}', '\u{0012}', '\u{0013}', '\u{009D}', '\u{0085}', '\u{0008}', '\u{0087}', // 10
    '\u{0018}', '\u{0019}', '\u{0092}', '\u{008F}', '\u{001C}', '\u{001D}', '\u{001E}', '\u{001F}', // 18
    '\u{0080}', '\u{0081}', '\u{0082}', '\u{0083}', '\u{0084}', '\u{000A}', '\u{0017}', '\u{001B}', // 20
    '\u{0088}', '\u{0089}', '\u{008A}', '\u{008B}', '\u{008C}', '\u{0005}', '\u{0006}', '\u{0007}', // 28
    '\u{0090}', '\u{0091}', '\u{0016}', '\u{0093}', '\u{0094}', '\u{0095}', '\u{0096}', '\u{0004}', // 30
    '\u{0098}', '\u{0099}', '\u{009A}', '\u{009B}', '\u{0014}', '\u{0015}', '\u{009E}', '\u{001A}', // 38
    '\u{0020}', '\u{00A0}', '\u{00E2}', '\u{00E4}', '\u{00E0}', '\u{00E1}', '\u{00E3}', '\u{00E5}', // 40
    '\u{00E7}', '\u{00F1}', '\u{005B}', '\u{002E}', '\u{003C}', '\u{0028}', '\u{002B}', '\u{0021}', // 48
    '\u{0026}', '\u{00E9}', '\u{00EA}', '\u{00EB}', '\u{00E8}', '\u{00ED}', '\u{00EE}', '\u{00EF}', // 50
    '\u{00EC}', '\u{00DF}', '\u{005D}', '\u{0024}', '\u{002A}', '\u{0029}', '\u{003B}', '\u{005E}', // 58
    '\u{002D}', '\u{002F}', '\u{00C2}', '\u{00C4}', '\u{00C0}', '\u{00C1}', '\u{00C3}', '\u{00C5}', // 60
    '\u{00C7}', '\u{00D1}', '\u{00A6}', '\u{002C}', '\u{0025}', '\u{005F}', '\u{003E}', '\u{003F}', // 68
    '\u{00F8}', '\u{00C9}', '\u{00CA}', '\u{00CB}', '\u{00C8}', '\u{00CD}', '\u{00CE}', '\u{00CF}', // 70
    '\u{00CC}', '\u{0060}', '\u{003A}', '\u{0023}', '\u{0040}', '\u{0027}', '\u{003D}', '\u{0022}', // 78
    '\u{00D8}', '\u{0061}', '\u{0062}', '\u{0063}', '\u{0064}', '\u{0065}', '\u{0066}', '\u{0067}', // 80
    '\u{0068}', '\u{0069}', '\u{00AB}', '\u{00BB}', '\u{00F0}', '\u{00FD}', '\u{00FE}', '\u{00B1}', // 88
    '\u{00B0}', '\u{006A}', '\u{006B}', '\u{006C}', '\u{006D}', '\u{006E}', '\u{006F}', '\u{0070}', // 90
    '\u{0071}', '\u{0072}', '\u{00AA}', '\u{00BA}', '\u{00E6}', '\u{00B8}', '\u{00C6}', '\u{00A4}', // 98
    '\u{00B5}', '\u{007E}', '\u{0073}', '\u{0074}', '\u{0075}', '\u{0076}', '\u{0077}', '\u{0078}', // A0
    '\u{0079}', '\u{007A}', '\u{00A1}', '\u{00BF}', '\u{00D0}', '\u{00DD}', '\u{00DE}', '\u{00AE}', // A8
    '\u{00A2}', '\u{00A3}', '\u{00A5}', '\u{00B7}', '\u{00A9}', '\u{00A7}', '\u{00B6}', '\u{00BC}', // B0
    '\u{00BD}', '\u{00BE}', '\u{00AC}', '\u{007C}', '\u{00AF}', '\u{00A8}', '\u{00B4}', '\u{00D7}', // B8
    '\u{007B}', '\u{0041}', '\u{0042}', '\u{0043}', '\u{0044}', '\u{0045}', '\u{0046}', '\u{0047}', // C0
    '\u{0048}', '\u{0049}', '\u{00AD}', '\u{00F4}', '\u{00F6}', '\u{00F2}', '\u{00F3}', '\u{00F5}', // C8
    '\u{007D}', '\u{004A}', '\u{004B}', '\u{004C}', '\u{004D}', '\u{004E}', '\u{004F}', '\u{0050}', // D0
    '\u{0051}', '\u{0052}', '\u{00B9}', '\u{00FB}', '\u{00FC}', '\u{00F9}', '\u{00FA}', '\u{00FF}', // D8
    '\u{005C}', '\u{00F7}', '\u{0053}', '\u{0054}', '\u{0055}', '\u{0056}', '\u{0057}', '\u{0058}', // E0
    '\u{0059}', '\u{005A}', '\u{00B2}', '\u{00D4}', '\u{00D6}', '\u{00D2}', '\u{00D3}', '\u{00D5}', // E8
    '\u{0030}', '\u{0031}', '\u{0032}', '\u{0033}', '\u{0034}', '\u{0035}', '\u{0036}', '\u{0037}', // F0
    '\u{0038}', '\u{0039}', '\u{00B3}', '\u{00DB}', '\u{00DC}', '\u{00D9}', '\u{00DA}', '\u{009F}', // F8
];

#[cfg(test)]
mod tests {
    use super::*;

    // ebcdic bytes for the given ascii string, via the cp500 table
    fn to_ebcdic(s: &str) -> Vec<u8> {
        s.chars()
            .map(|c| CP500.iter().position(|&m| m == c).unwrap() as u8)
            .collect()
    }

    #[test]
    fn detects_ebcdic_uppercase_and_digits() {
        // the regression the naive "is alphanumeric" rule gets wrong: under
        // latin-1 these decode to accented letters, which are alphanumeric
        for s in ["ACCOUNT", "0012345"] {
            let (text, enc) = decode(&to_ebcdic(s), EncodingMode::Auto);
            assert_eq!(enc, Encoding::Ebcdic, "{s}");
            assert_eq!(text, s);
        }
    }

    #[test]
    fn detects_ebcdic_mixed_case_and_punctuation() {
        for s in ["Sample AFP file  ", "201006-CURRENT", "not defined"] {
            let (text, enc) = decode(&to_ebcdic(s), EncodingMode::Auto);
            assert_eq!(enc, Encoding::Ebcdic, "{s}");
            assert_eq!(text, s);
        }
    }

    #[test]
    fn keeps_ascii_with_early_punctuation() {
        // these are the cases a strict "all alphanumeric" probe flips to
        // ebcdic by mistake
        for s in [
            "Real ASCII NOP here",
            "ACCOUNT_ID=12345",
            "2010-06-01 CURRENT",
            "{\"k\":\"v\"}",
        ] {
            let (text, enc) = decode(s.as_bytes(), EncodingMode::Auto);
            assert_eq!(enc, Encoding::Ascii, "{s}");
            assert_eq!(text, s);
        }
    }

    #[test]
    fn ebcdic_pad_bytes_do_not_confuse_the_probe() {
        let mut bytes = to_ebcdic("AB");
        bytes.extend(std::iter::repeat_n(0x40, 20));
        let (text, enc) = decode(&bytes, EncodingMode::Auto);
        assert_eq!(enc, Encoding::Ebcdic);
        assert_eq!(text.trim_end(), "AB");
    }

    #[test]
    fn forced_modes_skip_the_probe() {
        let bytes = to_ebcdic("ACCOUNT");
        assert_eq!(decode(&bytes, EncodingMode::Ascii).1, Encoding::Ascii);
        assert_eq!(decode(b"plain", EncodingMode::Ebcdic).1, Encoding::Ebcdic);
    }

    #[test]
    fn empty_and_blank_input_defaults_to_ascii() {
        assert_eq!(decode(b"", EncodingMode::Auto).1, Encoding::Ascii);
        assert_eq!(decode(b"    ", EncodingMode::Auto).1, Encoding::Ascii);
    }

    #[test]
    fn escapes_control_bytes() {
        assert_eq!(escape("a\tb\u{0}c"), "a\\tb\\x00c");
    }
}
