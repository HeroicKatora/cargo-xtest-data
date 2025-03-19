# Usage for repository authors

This section is aimed at best practices, as far as known, for setting up a
repository using this crate. For applying this to Rust crates, see the readme
section [How to apply](../Readme.md#How-to-apply). For API usage consult [the
documentation](https://docs.rs/xtest-data/).

You can run the xtask against the repository to test your current state without
going through CI and before distribution. Note that it will detect that it is
running against a dirty state by checking the existence of a normalized commit
hash in the crate file's VCS information file.

```bash
cargo run --bin xtask --features bin-xtask -- \
  --path to/your/crate test
```

The xtask will:
1. Run `cargo package` to create the `.crate` archive and accompanying pack
   directory. Note that this requires the sources selected for the crate to be
   unmodified.
2. Stop, if `test` is not selected. Otherwise, decompress and unpack this
   archive into a temporary directory.
3. Compile the package with `xtest-data` overrides for local development (see
   next section). In particular: `CARGO_XTEST_DATA_PACK_OBJECTS` will point to
   the pack output directory; `CARGO_XTEST_DATA_TMPDIR` will be set to a
   temporary directory create within the `target` directory; `CARGO_TARGET_DIR`
   will also point to the target directory.

This all ensure we can keep some `rustc` cached data around while otherwise
simulating a fresh distribution compilation.
