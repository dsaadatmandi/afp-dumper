# afp-dumper

splits AFP files into smaller files at document boundaries

## build step

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

output is named `{input}_{index}.afp`.

## how

runs 2 passes

**pass 1** streams through the file with an aho-corasick automaton searching for AFP structured field markers: BDT (`0xD3A8A8`), EDT (`0xD3A9A8`), BPG (`0xD3A8AF`), and EPG (`0xD3A9AF`), the state machine tracks the current position in the document structure (outside document, inside document, inside page) and emits document start/end events

before the first BDT, any bytes are treated as the resource preamble (fonts, overlays, page segments), this preamble is read once and prepended to every output file so that resources are available to all split documents

**pass 2** groups the document spans from pass 1 into chunks that fit within the max size limit, it then re-opens the input file and copies the preamble and document byte ranges into each output file via seek + copy

