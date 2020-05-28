# suffine

[![build](https://github.com/mosmeh/suffine/workflows/build/badge.svg)](https://github.com/mosmeh/suffine/actions)

Suffix array construction for huge strings that require space larger than available memory

WIP. Take a look at the examples!

## Examples

```rust
use suffine::IndexBuilder;

let text = "I scream, you scream, we all scream for ice cream!";
let index = IndexBuilder::new(text)
    .block_size(1024 * 1024)
    .build()
    .unwrap();
assert_eq!(index.positions("cream"), &[30, 44, 15, 3]);
```

Or you can directly build on a disk:

```rust
use std::fs::File;
use std::io::BufWriter;

let writer = BufWriter::new(File::create("index").unwrap());
IndexBuilder::new(text)
    .block_size(1024 * 1024)
    .build_to_writer_native_endian(writer)
    .unwrap();
```

Later you can load the index:

```rust
use std::fs;
use suffine::Index;

let bytes = fs::read("index").unwrap();
let index = Index::from_bytes(text, &bytes);
```

suffine also has `MultiDocIndex`:

```rust
use suffine::MultiDocIndexBuilder;

let text = "Roses are red,
Violets are blue,
sugar is sweet,
And so are you.";
let index = IndexBuilder::new(text).build().unwrap();
let multi_doc_index = MultiDocIndexBuilder::new(index)
    .delimiter('\n')
    .build()
    .unwrap();
let result = multi_doc_index
    .doc_positions("are")
    .collect::<Vec<(u32, u32)>>();
assert_eq!(result, [(1, 8), (0, 6), (3, 7)]);
```
