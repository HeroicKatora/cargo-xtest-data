# Known problems

## Superfluous git interactions

When fetching data, git may repeatedly ask for credentials and is pretty slow.
This issue should not occur when `git` supports `sparse-checkout`. This is
because we are shelling out to Git and `git checkout`, which we utilize to very
selectively unshallow the commit at the exact path specs which we require, does
not keep the connection alive—even when you give it multiple pathspecs at the
same time through `--pathspecs-from-file=-`. With `sparse-checkout`, however,
we only call this once which lowers the number of connection attempts. A
workaround is to setup a local agent and purge that afterwards or to create a
short-lived token instead.
