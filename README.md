# afp-dumper

Command line utility to interact efficiently with large AFP files

Features:
- Indexes the file — how many documents, page groups and pages it holds
- Splits AFP along page group boundaries
- Dumps TLE attribute names and values, decoded from their triplets
- Dumps NOP content, which the architecture leaves free-form
- Reports the position of everything it dumps, e.g. `doc 3 group 1 page 4`

## build

```
cargo build --release
```

## usage

indexing always runs, everything else is opt-in

```
afp-dumper -i input.afp
    index only — no files written, nothing dumped

afp-dumper -i input.afp -m 1048576
    also split into output/ at page group boundaries

afp-dumper -i input.afp --dump-all --dump-out tags.txt
    also dump every NOP and TLE, with its position, to tags.txt
```

| Flag | Description |
|------|-------------|
| `-i` | path to the input file (required) |

splitting, off unless `-m` is given:

| Flag | Description |
|------|-------------|
| `-m` | approx max output file size in bytes (splits before reaching) |
| `-o` | output directory (default: `output`) |

inspection, off by default:

| Flag | Description |
|------|-------------|
| `--dump-nop` | dump the content of every NOP |
| `--dump-tle` | dump the attribute name and value of every TLE |
| `--dump-all` | both of the above |
| `--dump-out` | write the dump to a file instead of the terminal |
| `--dump-limit` | records to print to the terminal before refusing (default: 1000) |
| `--encoding` | `auto`, `ascii` or `ebcdic` (default: `auto`) |

when `-m` is given, output is named `{input}_{index}.afp`

dump output is one line per record: its position, and the structured field's
data. nothing from the introducer, and nothing describing the field

```
preamble: NOP Sample AFP file
doc 0 group 0: TLE ACCOUNT = 0012345
doc 0 group 0 page 0: NOP Plain ASCII NOP 42
doc 1 page 0: TLE SECOND = doc-one
```

group and page indexes restart at every document, and the group is left out for
documents that have no page groups. records before the first BDT are `preamble`

## encoding

encoding is detected per record from the bytes themselves, because it is
arbitrary in practice and both encodings can appear inside a single file

`--encoding auto` reads the first 10 bytes that are not space, NUL or
whitespace, and uses ascii if every one of them is printable ascii, otherwise
cp500 ebcdic. this works because ebcdic letters, digits and its `X'40'` pad all
sit at or above `0x80`

a code page declared in the data is never consulted, and the detection is not
reconciled against it. a field that declares one encoding and holds another
decodes as what it actually holds

`--encoding ascii` or `--encoding ebcdic` forces one and skips the check

## How does this work

runs 2 passes

**pass 1** streams through the file with an aho-corasick automaton searching for AFP structured field markers: BDT (`0xD3A8A8`), EDT (`0xD3A9A8`), BNG (`0xD3A8AD`), ENG (`0xD3A9AD`), BPG (`0xD3A8AF`), and EPG (`0xD3A9AF`), the state machine tracks the current position in the document hierarchy and emits document start/end and page group start/end events

when a dump is requested, NOP (`0xD3EEEE`) and TLE (`0xD3A090`) join the same automaton, so finding them costs nothing extra — they are not compiled in otherwise

bytes before the first BDT are treated as the resource preamble (fonts, overlays, page segments), this preamble is read once and prepended to every output file so that resources are available to all split outputs

if the preamble contains a BPF (`0xD3A8A5`), each output file is closed with EPF (`0xD3A9A5`)

**pass 1b** only runs when dumping. a marker gives an offset but not a length, so each NOP/TLE hit is seeked to and its structured field decoded there: the `0x5A` framing byte (sniffed once, since it is platform framing rather than architecture), the 2-byte SFLength, the flag byte, then any SFI extension skipped and any trailing padding stripped. the file is never walked record by record, so a multi-gigabyte file still costs one sequential read plus one seek per hit

the same three bytes can occur inside image or presentation text payloads, and type code `0xEE` is *Data*, so NOP sits right next to IPD (`0xD3EEFB`), PTX (`0xD3EE9B`) and OCD (`0xD3EE92`). every hit is validated — framing byte present, plausible SFLength, record ending inside the file and on the next record's frame — and rejected hits are counted, not fatal

TLE content is entirely triplets, of which two carry the tag itself: `X'02'` fully qualified name (type `X'0B'`, the attribute name) and `X'36'` attribute value. every other triplet describes the field rather than carrying content — `X'01'` declares a code page, `X'80'` is an attribute qualifier — and is stepped over. NOP has no triplets, so its body is dumped as text

**pass 2** only runs when `-m` is given. it assembles documents and their page groups from the events, each output file is a complete AFP print file containing one document with one or more page groups, wrapped in BDT/EDT

when the source has no page groups (no BNG/ENG), each document is treated as a single virtual page group and split at document boundaries, when page groups exist, split at ENG boundaries, a page group is never split internally

document-level content (environment groups, resource groups between BDT and the first BNG) is replicated into each output file that contains page groups from that document, the writer uses seek + copy — nothing is buffered in memory
