# Customization points for packagers

In all settings, the `xtest_data` binary will inspect the following:
* The `Cargo.toml` file located in the `CARGO_MANIFEST_DIR` will be read,
  decoded and must at least contain the keys `package.name`, `package.version`,
  `package.repository`.

The `xtest_data` library will read the following environment variables:

* `CARGO_XTEST_DATA_TMPDIR` (fallback: `TMPDIR`) is required to be set when any
  of the tests are _NOT_ integration tests. Simply put, the setup creates some
  auxiliary data files but it can not guarantee cleaning them up. This makes an
  explicit effort to communicate this to the environment. Feel free to contest
  this reasoning if you feel your use-case were better addressed with an
  implicit, leaking temporary directory.
* `CARGO_XTEST_DATA_PACK_OBJECTS`: A directory for git pack objects (see `man
  git pack-objects`). Pack files are written to this directory when running
  tests from source, and read from this directory when running tests from a
  `.crate` archive. These are the same objects that would be fetched when doing
  a shallow  and sparse clone from the source repository.
* `CARGO_XTEST_VCS_INFO`: Path to a file with version control information as
  json, equivalent in structure to cargo's generated VCS information. This will
  force xtest into VCS mode, where resources are replaced with data from the
  pack object(s). Can be used to either force crates to supply internal vcs
  information or to supplement such information. For example, packages
  generated with `cargo package --allow-dirty` will not include such a file,
  and this can be used to override with a forced selection.
