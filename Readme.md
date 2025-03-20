Fetch auxiliary test data when testing published crates.

# What this library is

This library addresses the problem that integration test suites and
documentation tests can not be ran from the published `.crate` archive alone,
if they depend on auxiliary data files that should not be shipped to downstream
packages and end users.

- [1. What this library is](#what-this-library-is)
  - [1.1 How to apply](#how-to-apply)
  - [1.2 Testing with the reference implementation](#how-to-test-crates)
  - [1.3 Testing without the binary](#how-to-test-without-the-binary)
- [2. Details](#details)

## How to apply

Integrate this package as a dev-dependency into your tests.

```bash
cargo add --dev xtest-data
```

```rust,ignore
let mut path = PathBuf::from("tests/data.zip");
xtest_data::setup!()
    .rewrite([&mut path])
    .build();
// Path is dynamically adjusted by xtest-data.
assert!(path.exists(), "{}", path.display());

// Consume test data files relative to that root instead.
path.join("my-test-file.bin");
```

Then add metadata into your package that describes how to fetch data archives
from the CI/CD system for published packages. This step is highly recommended
so that self-described testing of the packaged crate with the xtask binary is
possible. You achieve this in the `Cargo.toml` file:

```toml
[package.metadata.xtest-data]
pack-archive = "tar:gz"
# Matches the Github Actions workflow file.
pack-artifact = "{repository}/releases/download/v{version}/xtest-data.tar.gz"
# Path create export data, and expect pack objects.
pack-objects = "target/xtest-data-pack"
```

For a corresponding example CI setup, see <.github/workflows/release.yml>.

## How to test crates

This repository contains a reference implementation for interpreting the
auxiliary metadata.

```bash
cargo install xtest-data --features="bin-xtask"

# test for developers
cargo-xtest-data test <file-path-to-repo>
# test for packager
cargo-xtest-data test-crate <crate>
# prepare a test but delay its execution
eval `cargo-xtest-data fetch <crate>`
```

For an offline use, archives can be handled as files:

```bash
# Prepare .crate and .xtest-data archives:
cargo-xtest-data pack
# on stdout, e.g.: ./target/xtest-data/xtest-data-1.0.0-beta.3.xtest-data

# < Upload/download/exchange archives >

# After downloading both files again:
eval `cargo-xtest-data \
  fetch xtest-data-1.0.0-beta.3.crate \
  --pack-artifact xtest-data-1.0.0-beta.3.xtest-data`
# Now proceed with regular testing
```

## How to test without the binary

The library component consumes its dependencies via environment variables and
the file system. The binary only does the job of orchestrating the file
preparation and execution with the corresponding settings. First, create the
self-contained git-object-pack collection with your test runs.

```bash
CARGO_XTEST_DATA_PACK_OBJECTS="$(pwd)/target/xtest-data" cargo test
zip xtest-data.zip -r target/xtest-data
```

This allows utilizing the library component to provide a compelling experience
for testing distributed packages with the test data as a separate archive. You
can of course pack `target/xtest-data` in any other shape or form you prefer.
When testing a crate archive reverse these steps:

```bash
unzip xtest-data.zip
CARGO_XTEST_DATA_PACK_OBJECTS="$(pwd)/target/xtest-data" cargo test
```

# Details

See the documentation folder.

- [Usage for crate authors](./docs/usage-for-repository-authors.md)
- [Customization points for packagers](./docs/customization-points-for-packagers.md)
- [Known problems](./docs/known-problems.md)
- [Goals and evaluation](./docs/goals-and-evaluation.md)
- [Overview of mode of action](./docs/overview-of-mode-of-action.md)

[cargo-xtask]: https://github.com/matklad/cargo-xtask
