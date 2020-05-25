# suffine CLI

Command-line interface for suffine

## Usage

### Index

```sh
suffine index foo.txt
```

The index will be created at `foo.suffine-index` in the same directory as `foo.txt`.

If your computer doesn't have enough memory, you can specify a block size in MB.

```sh
suffine index foo.txt -b 1024
```

It will eat roughly 5 times the block size of memory.

### Search

```sh
suffine search foo.txt -q "blah blah" -n 5
```

The first 5 hits are shown in an arbitrary order.
