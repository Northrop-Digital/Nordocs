//! End-to-end CLI tests for the `ndoc` binary.

use assert_cmd::Command;
use predicates::prelude::*;

/// `ndoc --help` runs and lists the refined command surface.
#[test]
fn help_lists_commands() {
    let mut cmd = Command::cargo_bin("ndoc").expect("ndoc binary builds");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("doc"));
}

/// `ndoc --version` prints a version string.
#[test]
fn version_prints() {
    let mut cmd = Command::cargo_bin("ndoc").expect("ndoc binary builds");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("ndoc"));
}

/// The `--json` envelope flag on a `doc` subcommand emits a JSON ok envelope.
#[test]
fn doc_json_envelope() {
    let mut cmd = Command::cargo_bin("ndoc").expect("ndoc binary builds");
    cmd.args(["doc", "outline", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));
}
