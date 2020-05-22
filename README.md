# suffine

[![build](https://github.com/mosmeh/suffine/workflows/build/badge.svg)](https://github.com/mosmeh/suffine/actions)

Suffix array construction for huge strings that require space larger than available memory

WIP. Take a look at examples!

## Example

```rust
use suffine::IndexBuilder;

fn main() {
    let text = "I scream, you scream, we all scream for ice cream!";
    let index = IndexBuilder::new(text)
        .block_size(1024 * 1024)
        .build()
        .unwrap();
    assert_eq!(index.find_positions("cream"), &[30, 44, 15, 3]);
}
```
