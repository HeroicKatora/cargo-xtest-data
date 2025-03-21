use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::inconclusive;

/// How we access `git` repositories.
#[derive(Debug)]
pub(crate) struct Git {
    bin: PathBuf,
}

/// A bare repository created by us.
pub(crate) struct ShallowBareRepository {
    path: PathBuf,
}

/// The repository containing the manifest of the crate to integration test.
pub(crate) struct CrateDir {
    path: PathBuf,
}

pub(crate) struct FileWaitLock {
    lock: std::fs::File,
}

pub(crate) struct Origin {
    pub url: OsString,
}

/// A git commit ID.
/// This is treated as opaque string data. Usually it's a Sha1 hash (20 byte, hex-encoded).
#[derive(Debug)]
pub(crate) struct CommitId(String);

pub(crate) enum PathSpec<'lt> {
    Path(&'lt Path),
}

impl Git {
    pub fn new() -> Result<Self, impl std::fmt::Display> {
        which::which("git").map(|bin| Git { bin })
    }

    /// Prepare `path` as a shallow clone of `origin`.
    /// Aborts if this isn't possible (see error handling policy).
    pub fn bare(&self, path: PathBuf, head: &CommitId) -> ShallowBareRepository {
        let repo = ShallowBareRepository { path };

        let _lock = FileWaitLock::for_git_dir(&repo.path);
        let mut cmd = repo.exec(self);

        if !repo.path.exists() {
            cmd.args(["init", "--bare", "--"]);
            cmd.arg(&repo.path);
        } else {
            // Test that the repo in fact exists and is recognized by git.
            cmd.args(["symbolic-ref", "HEAD"]);
        }

        cmd.status()
            .unwrap_or_else(|mut err| inconclusive(&mut err));

        let content = format!("{}\n", head.0);
        std::fs::write(repo.path.join("shallow"), content)
            .unwrap_or_else(|mut err| inconclusive(&mut err));

        repo
    }
}

impl From<&'_ str> for CommitId {
    fn from(st: &'_ str) -> CommitId {
        let st = st.trim();
        assert!(
            st.len() >= 40,
            "Unlikely to be a safe Git Object ID in vcs pin file: {}",
            st
        );
        CommitId(st.to_owned())
    }
}

impl CrateDir {
    pub fn new(path: &str, git: &Git) -> Self {
        let dir = CrateDir {
            path: Path::new(path).to_owned(),
        };

        let mut cmd = dir.exec(git);
        cmd.args(["status", "--short"]);
        cmd.status()
            .unwrap_or_else(|mut err| inconclusive(&mut err));
        dir
    }

    pub fn exec(&self, git: &Git) -> Command {
        let mut cmd = Command::new(&git.bin);
        cmd.current_dir(&self.path);
        // Ensure we open _no_ handles.
        // Override this later if necessary.
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        cmd
    }

    pub fn tracked(&self, git: &Git, paths: &mut dyn Iterator<Item = PathSpec<'_>>) {
        let mut cmd = self.exec(git);
        cmd.stdout(Stdio::piped());
        cmd.args([
            "status",
            "--no-renames",
            "--ignored=matching",
            "--porcelain=v2",
            "--short",
            "-z",
        ]);
        cmd.arg("--");
        let mut any = false;
        cmd.args(paths.map(|st| {
            any = true;
            st.to_string()
        }));

        if !any {
            return;
        }

        let output = cmd
            .output()
            .unwrap_or_else(|mut err| inconclusive(&mut err));
        let items =
            String::from_utf8(output.stdout).unwrap_or_else(|mut err| inconclusive(&mut err));
        for item in items.split('\0') {
            if item.starts_with('!') {
                eprintln!("{}", item);
                inconclusive(&mut "Your test depends on ignored file(s)");
            } else if item.starts_with('?') {
                eprintln!("{}", item);
                inconclusive(&mut "Your test depends on untracked file(s)");
            }
        }
    }

    pub fn pack_objects(
        &self,
        git: &Git,
        paths: &mut dyn Iterator<Item = PathSpec<'_>>,
        pack_name: OsString,
    ) {
        let _lock = FileWaitLock::for_git_dir(&self.path);

        let PathSpecFilter {
            simple_filter,
            complex_paths,
        } = paths.collect();
        let sparse = self.sparse_rev_list(git, &simple_filter);

        if !complex_paths.is_empty() {
            inconclusive(&mut "Sorry, paths too complex to pack reliably");
        }

        let mut cmd = self.exec(git);
        cmd.args(["pack-objects"]);
        cmd.arg(Path::new(&pack_name).join("xtest-data"));
        cmd.stdin(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut running = cmd.spawn().unwrap_or_else(|mut err| inconclusive(&mut err));
        let stdin = running.stdin.as_mut().expect("Spawned with stdio-piped");
        std::io::Write::write_all(stdin, &sparse).unwrap_or_else(|mut err| inconclusive(&mut err));
        running.stdin = None;

        let exit = running
            .wait_with_output()
            .unwrap_or_else(|mut err| inconclusive(&mut err));

        if !exit.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&exit.stderr));
            inconclusive(&mut "Git operation was not successful");
        }
    }

    fn sparse_rev_list(&self, git: &Git, paths: &[PathSpec<'_>]) -> Vec<u8> {
        let CommitId(oid) = self
            .hash_sparse_oid(git, paths)
            .unwrap_or_else(|mut err| inconclusive(&mut err));

        let list_for = |filterspec| {
            let mut cmd = self.exec(git);
            // Shallow, and sparse filtered, list of objects.
            cmd.args(["rev-list", "-n", "1", "--objects", "--no-object-names"]);
            cmd.arg(filterspec);
            cmd.arg("HEAD");
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let exit = cmd
                .output()
                .unwrap_or_else(|mut err| inconclusive(&mut err));
            if !exit.status.success() {
                eprintln!("{}", String::from_utf8_lossy(&exit.stderr));
                inconclusive(&mut "Git operation was not successful");
            }

            exit.stdout
        };

        let mut objects = list_for(format!("--filter=sparse:oid={oid}", oid = oid));
        let mut treeish = list_for("--filter=blob:none".into());

        objects.append(&mut treeish);
        objects
    }

    fn hash_sparse_oid(&self, git: &Git, paths: &[PathSpec<'_>]) -> std::io::Result<CommitId> {
        let mut cmd = self.exec(git);
        cmd.args(["hash-object", "-w", "--stdin"]);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut running = cmd.spawn().unwrap_or_else(|mut err| inconclusive(&mut err));
        let stdin = running.stdin.as_mut().expect("Spawned with stdio-piped");
        for path in paths {
            use std::io::Write;
            let path = path.as_encompassing_path().expect("Only simple paths");
            write!(stdin, "{}\0", path.display()).unwrap_or_else(|mut err| inconclusive(&mut err));
        }

        running.stdin = None;
        let exit = running
            .wait_with_output()
            .unwrap_or_else(|mut err| inconclusive(&mut err));

        if !exit.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&exit.stderr));
            // We try to do some detection here, because this is the first command that is ran when
            // we assume to be in a git directory.
            inconclusive_but_maybe_gitdir(&exit, &mut "Git operation was not successful");
        }

        let id = String::from_utf8_lossy(&exit.stdout);
        Ok(id.as_ref().into())
    }
}

#[derive(Default)]
struct PathSpecFilter<'lt> {
    simple_filter: Vec<PathSpec<'lt>>,
    complex_paths: Vec<PathSpec<'lt>>,
}

impl<'lt> Extend<PathSpec<'lt>> for PathSpecFilter<'lt> {
    fn extend<T: IntoIterator<Item = PathSpec<'lt>>>(&mut self, paths: T) {
        let simple_filter = &mut self.simple_filter;
        let complex = paths.into_iter().filter_map(|path| {
            if let Some(sparse_compatible) = path.as_encompassing_path() {
                // Look, we don't have proper escaping for it yet and no NUL separator.
                let format = sparse_compatible.display().to_string();
                // Assuming that this is fine.
                if !format.contains('\n') && !format.contains('\0') {
                    simple_filter.push(path);
                    return None;
                }

                Some(path)
            } else {
                Some(path)
            }
        });
        self.complex_paths.extend(complex);
    }
}

impl<'lt> std::iter::FromIterator<PathSpec<'lt>> for PathSpecFilter<'lt> {
    fn from_iter<T: IntoIterator<Item = PathSpec<'lt>>>(iter: T) -> Self {
        let mut this = PathSpecFilter::default();
        this.extend(iter);
        this
    }
}

impl ShallowBareRepository {
    pub fn exec(&self, git: &Git) -> Command {
        let mut cmd = Command::new(&git.bin);
        cmd.arg("--git-dir");
        cmd.arg(&self.path);
        // Ensure we open _no_ handles.
        // Override this later if necessary.
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());
        cmd
    }

    pub fn unpack(&self, git: &Git, packs: &OsString) {
        let _lock = FileWaitLock::for_git_dir(&self.path);

        let opendir = std::fs::read_dir(packs).unwrap_or_else(|mut err| inconclusive(&mut err));

        for entry in opendir.filter_map(Result::ok) {
            if !entry
                .path()
                .to_str()
                .map_or(false, |st| st.ends_with("pack"))
            {
                continue;
            }

            let mut file =
                std::fs::File::open(entry.path()).unwrap_or_else(|mut err| inconclusive(&mut err));

            let mut git = self.exec(git);
            git.args(["unpack-objects", "-r"]);
            git.stdin(Stdio::piped());

            let mut cmd = git.spawn().unwrap_or_else(|mut err| inconclusive(&mut err));
            let mut stdin = cmd.stdin.as_mut().expect("Supplied with Stdio::piped");

            std::io::copy(&mut file, &mut stdin).unwrap_or_else(|mut err| inconclusive(&mut err));
            std::io::Write::flush(stdin).unwrap_or_else(|mut err| inconclusive(&mut err));
            // Flush and close.
            cmd.stdin = None;

            let exit = cmd
                .wait_with_output()
                .unwrap_or_else(|mut err| inconclusive(&mut err));
            if !exit.status.success() {
                eprintln!("{}", String::from_utf8_lossy(&exit.stderr));
                inconclusive(&mut "Git operation was not successful");
            }
        }
    }

    // Known false positive in initializatioon of `complex_paths`.
    // We need to take ownership of `path` in a branch.
    #[allow(clippy::unnecessary_filter_map)]
    pub fn checkout(
        &self,
        git: &Git,
        worktree: &Path,
        head: &CommitId,
        paths: &mut dyn Iterator<Item = PathSpec<'_>>,
    ) {
        let _lock = FileWaitLock::for_git_dir(&self.path);

        let PathSpecFilter {
            simple_filter,
            complex_paths,
        } = paths.collect();

        let mut cmd = self.exec(git);
        cmd.args(["worktree", "add", "--no-checkout"]);
        cmd.arg(worktree);
        cmd.arg(head);
        let exit = cmd
            .output()
            .unwrap_or_else(|mut err| inconclusive(&mut err));
        if !exit.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&exit.stderr));
            inconclusive(&mut "Git operation was not successful");
        }

        // First setup sparse-checkout
        // Note that this is in beta and not supported, so let's fallback if necessary.
        let try_sparse_checkout = || -> std::io::Result<()> {
            let mut cmd = self.exec(git);
            cmd.arg("--work-tree");
            cmd.arg(worktree);
            cmd.args(["sparse-checkout", "--no-cone", "set", "--stdin"]);
            cmd.stdin(Stdio::piped());
            let mut running = cmd.spawn()?;
            let stdin = running.stdin.as_mut().expect("Spawned with stdio-piped");
            for path in &simple_filter {
                let simple = path.as_encompassing_path().unwrap().display().to_string();
                use std::io::Write;
                // > This includes interpreting pathnames that begin with a double quote (") as C-style quoted strings.
                // Since there is no NUL separation (yet?) we use this.
                writeln!(stdin, "{}", simple).unwrap_or_else(|mut err| inconclusive(&mut err));
            }
            running.stdin = None;
            let exit = running.wait_with_output()?;
            if !exit.status.success() {
                return Err(std::io::ErrorKind::Other.into());
            }
            Ok(())
        };

        if let Err(err) = try_sparse_checkout() {
            eprintln!(
                "Version of Git appears to not support sparse-checkout: {}",
                err
            );
            let mut all_again = simple_filter.into_iter().chain(complex_paths);
            return self.checkout_fallback_slow(git, worktree, head, &mut all_again);
        }

        let mut cmd = self.exec(git);
        cmd.arg("--work-tree");
        cmd.arg(worktree);
        cmd.arg("checkout");
        cmd.arg("--force");
        cmd.arg(&head.0);
        cmd.stderr(Stdio::piped());
        let exit = cmd
            .output()
            .unwrap_or_else(|mut err| inconclusive(&mut err));

        if !exit.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&exit.stderr));
            inconclusive(&mut "Git operation was not successful");
        }

        self.checkout_fallback_slow(git, worktree, head, &mut complex_paths.into_iter());
    }

    /// A version of `checkout` that uses checkout and a list pathspecs from stdin to determine the
    /// files in the worktree. However, it appears that this cases git to open a connection to the
    /// remote _for every single one_.
    pub fn checkout_fallback_slow(
        &self,
        git: &Git,
        worktree: &Path,
        head: &CommitId,
        paths: &mut dyn Iterator<Item = PathSpec<'_>>,
    ) {
        let mut cmd = self.exec(git);
        cmd.arg("--work-tree");
        cmd.arg(worktree);
        cmd.args(["checkout", "--no-guess", "--force"]);
        cmd.args(["--pathspec-from-file=-", "--pathspec-file-nul"]);
        cmd.arg(&head.0);
        cmd.stdin(Stdio::piped());
        let mut running = cmd.spawn().unwrap_or_else(|mut err| inconclusive(&mut err));
        let stdin = running.stdin.as_mut().expect("Spawned with stdio-piped");
        for path in paths {
            use std::io::Write;
            write!(stdin, "{}\0", path).unwrap_or_else(|mut err| inconclusive(&mut err));
        }
        running.stdin = None;
        let exit = running
            .wait_with_output()
            .unwrap_or_else(|mut err| inconclusive(&mut err));
        if !exit.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&exit.stderr));
            inconclusive(&mut "Git operation was not successful");
        }
    }
}

fn inconclusive_but_maybe_gitdir(
    output: &std::process::Output,
    descriptor: &mut dyn std::fmt::Display,
) {
    struct WithSuspicionNoVcsPresent<'lf>(&'lf mut dyn std::fmt::Display);

    impl std::fmt::Display for WithSuspicionNoVcsPresent<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "{}", self.0)?;
            writeln!(
                f,
                "The xtest-data setup expected to find the crate within a git directory."
            )?;
            writeln!(f, "This is because it did not find a `.cargo_vcs_info.json` file sibling to `Cargo.toml`.")?;
            writeln!(
                f,
                "If you've unpacked a `.crate` archive, are you certain it did contain the file?."
            )?;
            Ok(())
        }
    }

    if let Ok(st) = std::str::from_utf8(&output.stderr) {
        // Failing to find a git repository looks something like this:
        //
        // fatal: Kein Git-Repository (oder irgendein Elternverzeichnis bis zum Einhängepunkt /)
        // Stoppe bei Dateisystemgrenze (GIT_DISCOVERY_ACROSS_FILESYSTEM nicht gesetzt).

        if st.contains("fatal")
            && st.contains("/)")
            && st.contains("GIT_DISCOVERY_ACROSS_FILESYSTEM")
        {
            inconclusive(&mut WithSuspicionNoVcsPresent(descriptor));
        }
    }

    inconclusive(descriptor);
}

impl FileWaitLock {
    pub fn for_git_dir(path: &Path) -> Self {
        use fs2::FileExt;
        let fslock_path = path
            .parent()
            .expect("Clone directory should not be root")
            .join("xtest-data.lock");

        let lock =
            std::fs::File::create(&fslock_path).unwrap_or_else(|mut err| inconclusive(&mut err));
        lock.lock_exclusive()
            .unwrap_or_else(|mut err| inconclusive(&mut err));

        FileWaitLock { lock }
    }
}

impl Drop for FileWaitLock {
    fn drop(&mut self) {
        if let Err(_) = fs2::FileExt::unlock(&self.lock) {
            // Otherwise we'd block indefinitely in this process?
            std::process::abort();
        }
    }
}

impl PathSpec<'_> {
    /// For git sparse checkout.
    pub fn as_encompassing_path(&self) -> Option<&Path> {
        match self {
            PathSpec::Path(path) => Some(path),
            // Should return None for a glob-filtered path since that is not supported.
        }
    }
}

impl std::convert::AsRef<OsStr> for CommitId {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl core::fmt::Display for PathSpec<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            PathSpec::Path(path) => write!(f, ":(top,literal){}", path.display()),
        }
    }
}
