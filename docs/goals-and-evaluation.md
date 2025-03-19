# Motivation

As a developer of a library, you will write some integration with the goal of
ensuring correct functionality of your code. Typically, these will be executed
in a CI pipeline before release. However, what if someone else—e.g. an Open
Source OS distribution—wants to repackage your code? In some cases they might
need to perform simple, small modifications: rewrite dependencies, apply
compilation options like hardening flags, etc. After those modifications it's
unclear if the end product still conforms to its own expectations. Thus will
want to run the integration test suite again. That's where the library comes in.
It should ensure that:

* It is unobtrusive in that it does not require modification to the code that
  is used when included as a dependency.
* Tests should be reproducible from the packaged `.crate`, and an author can
  check this property locally and during pre-release checks.
* Auxiliary data files required for tests are referenced unambiguously.
* It does not make unmodifiable assumptions about the source of test data.

# Specific Goals and Evaluation

* The package shold be a pure dev-dependency and there is focus on introducing
  a small amount of dependencies. (Any patches to minimize this further are
  welcome. We might add a toggle to disable locks and its dependencies if
  non-parallel test execution is good enough?)
* A full offline mode with minimal auxiliary source archives is provided.
  Building the crate without executing tests does not require any test data.
* The binay tool can be used for local development and CI (we use it in our
  own pipeline for example). It's not strongly linked to the implementation,
  just the public interface, so it is possible to replace it with your own
  logic.
* Auxiliary files are strongly referenced through the commit object ID of the
  distributed crate, which implies a particular tree-ish from which they are
  retrieved.
* It is possible to overwrite the data source as long as it provides a git
  compatible server. For example, you might pre-clone the source commit and
  provide the data via a local `file://` repository. This is provided by the
  command line flag `--pack-artifact` of the `crate-test` subcommand and it
  accepting any packaged `.crate` archive.
