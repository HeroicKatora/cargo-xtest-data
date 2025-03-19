# Overview of mode of action

Read this file if you want to find out the 'how' of the implementation without
reading it. It should be mainly helpful for compatible alternative
implementations. Also please help expand it with points you wish to have known
if you do write one.

## How the binary works

For this task it augments `Cargo.toml` with additional fields that describe how
an artifact archive composed from VCS files that are associated with the exact
version at which they were created. The packed data and exact version is then
referenced when executing test from the `.crate` archive. We use a filtered
`git-pack-archive` here. This crates a bundle of raw git objects including the
files, tree-ishs and exact commit reference. The receiving side can then also
unpack those archives selectively to recreate the exact necessary file
structure with a sparse checkout.

## How testing can switch data source in the packed crate

The expectation of the library is that you access all data through this library
instead of as a direct path. Note even though APIs are typed as infallible they
will panic when something is missing.The reasoning is that this indicates a
faulty setup such as faulty data not corresponding to the indicated commit, not
something the test itself should handle. In the typical success case this will
add only minor code paths to test implementations.

When `cargo` packages a `.crate`, it will include a file called
`.cargo_vcs_info.json` which contains basic version information, i.e. the
commit ID that was used as the basis of creation of the archive. When the
methods of this crate run, they detect the presence or absence of this file to
determine if data can be fetched (we also detect the repository information
from `Cargo.toml`).

If we seem to be running outside the development repository, then by default we
won't do anything but validate the information, debug print what we _plan_ to
fetch—and then instantly panic. However, if the environment variable
`CARGO_XTEST_DATA_FETCH` is set to `yes`, `true` or `1` then we will try
to download and checkout requested files to the relative location.
