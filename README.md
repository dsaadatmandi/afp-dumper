# afp-dumper

splits AFP files into smaller files at page group boundaries

## build

```
cargo build --release
```

## usage

```
afp-dumper -i input.afp -o output_dir -m 1048576
```

| Flag | Required | Description |
|------|----------|-------------|
| `-i` | yes | path to the input file |
| `-o` | no | output directory (default: `output`) |
| `-m` | yes | approx max output file size in bytes (splits before reaching) |

output is named `{input}_{index}.afp`

## how

runs 2 passes

**pass 1** streams through the file with an aho-corasick automaton searching for AFP structured field markers: BDT (`0xD3A8A8`), EDT (`0xD3A9A8`), BNG (`0xD3A8AD`), ENG (`0xD3A9AD`), BPG (`0xD3A8AF`), and EPG (`0xD3A9AF`), the state machine tracks the current position in the document hierarchy and emits document start/end and page group start/end events

bytes before the first BDT are treated as the resource preamble (fonts, overlays, page segments), this preamble is read once and prepended to every output file so that resources are available to all split outputs

if the preamble contains a BPF (`0xD3A8A5`), each output file is closed with EPF (`0xD3A9A5`)

**pass 2** assembles documents and their page groups from the events, each output file is a complete AFP print file containing one document with one or more page groups, wrapped in BDT/EDT

when the source has no page groups (no BNG/ENG), each document is treated as a single virtual page group and split at document boundaries, when page groups exist, split at ENG boundaries, a page group is never split internally

document-level content (environment groups, resource groups between BDT and the first BNG) is replicated into each output file that contains page groups from that document, the writer uses seek + copy — nothing is buffered in memory
