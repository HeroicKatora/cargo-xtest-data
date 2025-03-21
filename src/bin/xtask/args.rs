use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(rename_all = "kebab-case")]
pub enum CargoXtestData {
    /// A subcommand to interact with `xtest-data` and cargo.
    XtestData {
        #[command(subcommand)]
        cmd: XtaskCommand,
    },
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(rename_all = "kebab-case")]
pub enum XtaskCommand {
    /// Integration test the xtest-data setup for a repository.
    ///
    /// This will:
    /// 1. Package the repository into a crate
    /// 2. Create the archive of test data
    /// 3. Unpack the crate into a temporary location
    /// 4. Prepare the test data from the archive
    /// 5. Run tests with the test data
    #[command(alias = "ci")]
    Test {
        /// The path to the source repository.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// If we should allow a dirty repository.
        ///
        /// This will fail to do the right thing if any test data is dirty. Unlike cargo, this
        /// will write a custom `vcs_info` file to use. However, all test data must a reachable
        /// within the tree given by the current VCS (otherwise it wouldn't be part of the pack).
        #[arg(long, default_value = "false")]
        allow_dirty: bool,
    },
    /// Test a published crate archive.
    ///
    /// This command fetches the linked data archives and executes the cargo test for that data.
    #[command(alias = "crate", alias = "crate-test")]
    TestCrate {
        /// A path to a `.crate` archive, or an unpacked version.
        ///
        /// The relevant difference to a source repository is the presence of a `.cargo_vcs_info`
        /// file that provides the stable reference to the exact VCS state. This file must be
        /// present and is automatically generated by cargo when publishing from a git repository.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Overwrite path to the downloaded `pack-artifact`.
        #[arg(id = "pack-artifact", long)]
        pack_artifact: Option<PathBuf>,
    },
    /// Pack the source data, but do not run the full integration test.
    ///
    /// This will only create the pack archive according to the instructions but it will not re-run
    /// the full test suite on a cleanroom unpack of the archive.
    #[command(alias = "package")]
    Pack {
        /// The path to the source repository.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// If we should allow a dirty repository.
        ///
        /// This will fail to do the right thing if any test data is dirty. Unlike cargo, this
        /// _will_ write a custom `vcs_info` file to use. However, all test data must a reachable
        /// within the tree given by the current VCS (otherwise it wouldn't be part of the pack).
        #[arg(long, default_value = "false")]
        allow_dirty: bool,
    },
    /// Run the data preparation step of a crate artifact.
    ///
    /// Downloads the artifacts if necessary and then unpacks them by running the suitable steps as
    /// defined by the archive's metadata. The output directory is suitable for use as a
    /// `CARGO_XTEST_DATA_PACK_OBJECTS` variable. The source file can be overridden by a path to a
    /// local file for offline and package manager use.
    ///
    /// Writes bash-formatted environment variables to stdout. No other output is produced except
    /// in stderr.
    #[command(alias = "fetch-artifacts")]
    Fetch {
        /// The path to the source crate archive.
        path: PathBuf,
        /// Provide a downloaded `pack-artifact`.
        #[arg(id = "pack-artifact", long)]
        pack_artifact: Option<PathBuf>,
        /// Provide an explicit write location. Otherwise, a default is chosen based on the crate
        /// name, version, and target directory.
        output: Option<PathBuf>,
    },
}
