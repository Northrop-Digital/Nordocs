//! End-to-end CLI tests for the `ndoc` binary.

use std::path::Path;

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

/// `ndoc build <fixture>` produces a non-empty PDF in the same directory.
#[test]
fn e2e_build_produces_pdf() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/sample.md");
    let expected_pdf = manifest.join("tests/fixtures/sample.pdf");

    // Remove any artifact from a previous run so this run's creation is asserted.
    let _ = std::fs::remove_file(&expected_pdf);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("build")
        .arg(&input)
        .assert()
        .success();

    assert!(
        expected_pdf.exists(),
        "expected PDF at {expected_pdf:?} was not created"
    );
    assert!(
        expected_pdf.metadata().expect("PDF metadata").len() > 0,
        "PDF at {expected_pdf:?} is empty"
    );
}

/// `ndoc build <missing>` exits non-zero and names the path in the error.
#[test]
fn e2e_build_missing_file() {
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", "nonexistent.md"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("nonexistent.md"));
}

/// `ndoc build --help` exits 0 and prints usage text.
#[test]
fn e2e_build_help() {
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Input Markdown"));
}

// ---------------------------------------------------------------------------
// E2E tests for document authoring commands (ndoc new / add / edit / build)
// ---------------------------------------------------------------------------

/// Return the absolute path of `p` as a `String` for use in `assert_cmd` args.
fn abs(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

/// `ndoc new <path>` creates the file and writes the ndoc document header.
#[test]
fn ndoc_new_creates_file() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("test.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .success();

    assert!(doc.exists(), "ndoc new must create the file");
    let content = std::fs::read_to_string(&doc).expect("read created file");
    assert!(
        content.contains("// ndoc document v1"),
        "created file must contain the ndoc document header"
    );
}

/// `ndoc new <path>` on an existing file exits non-zero and leaves the file unchanged.
#[test]
fn ndoc_new_rejects_existing() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("existing.ndoc.typ");
    std::fs::write(&doc, "original content").expect("write pre-existing file");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .failure();

    let content = std::fs::read_to_string(&doc).expect("read file after failed new");
    assert_eq!(
        content, "original content",
        "ndoc new must not overwrite an existing file"
    );
}

/// `ndoc add` with a unique name exits 0 and the entry appears in the document.
#[test]
fn ndoc_add_unique_name() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    let content_file = tmp.path().join("hero.typ");
    std::fs::write(&content_file, "#let hero = ()").expect("write content file");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .success();

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "add",
            &abs(&doc),
            "hero",
            "--kind",
            "component",
            "--content-file",
            &abs(&content_file),
        ])
        .assert()
        .success();

    let file_content = std::fs::read_to_string(&doc).expect("read document after add");
    assert!(
        file_content.contains("NDOC-ENTRY: hero"),
        "document must contain the new entry after add"
    );
}

/// `ndoc add` with a duplicate name exits non-zero and leaves the file unchanged.
#[test]
fn ndoc_add_duplicate_name() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    let content_file = tmp.path().join("comp.typ");
    std::fs::write(&content_file, "#let c = ()").expect("write content file");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .success();

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "add",
            &abs(&doc),
            "hero",
            "--content-file",
            &abs(&content_file),
        ])
        .assert()
        .success();

    let before = std::fs::read_to_string(&doc).expect("read document before duplicate add");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "add",
            &abs(&doc),
            "hero",
            "--content-file",
            &abs(&content_file),
        ])
        .assert()
        .failure();

    let after = std::fs::read_to_string(&doc).expect("read document after failed add");
    assert_eq!(
        before, after,
        "file must be unmodified after duplicate-entry error"
    );
}

/// `ndoc add` on a non-existent document exits non-zero.
#[test]
fn ndoc_add_missing_file() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let ghost = tmp.path().join("ghost.ndoc.typ");
    let content_file = tmp.path().join("comp.typ");
    std::fs::write(&content_file, "#let c = ()").expect("write content file");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "add",
            &abs(&ghost),
            "my-entry",
            "--content-file",
            &abs(&content_file),
        ])
        .assert()
        .failure();
}

/// `ndoc edit` on an existing entry exits 0 and replaces only that entry's content.
#[test]
fn ndoc_edit_existing_entry() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    let v1 = tmp.path().join("hero_v1.typ");
    let v2 = tmp.path().join("hero_v2.typ");
    std::fs::write(&v1, "#let hero = ()").expect("write original content");
    std::fs::write(&v2, "#let hero = (updated: true)").expect("write updated content");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .success();

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["add", &abs(&doc), "hero", "--content-file", &abs(&v1)])
        .assert()
        .success();

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["edit", &abs(&doc), "hero", "--content-file", &abs(&v2)])
        .assert()
        .success();

    let file_content = std::fs::read_to_string(&doc).expect("read document after edit");
    assert!(
        file_content.contains("#let hero = (updated: true)"),
        "edited entry must contain the updated content"
    );
    assert!(
        file_content.contains("// ndoc document v1"),
        "document header must be preserved after edit"
    );
}

/// `ndoc edit` on an absent entry name exits non-zero and leaves the file unchanged.
#[test]
fn ndoc_edit_missing_entry() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    let content_file = tmp.path().join("content.typ");
    std::fs::write(&content_file, "#let x = ()").expect("write content file");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .success();

    let before = std::fs::read_to_string(&doc).expect("read document before failed edit");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "edit",
            &abs(&doc),
            "nonexistent",
            "--content-file",
            &abs(&content_file),
        ])
        .assert()
        .failure();

    let after = std::fs::read_to_string(&doc).expect("read document after failed edit");
    assert_eq!(
        before, after,
        "file must be unmodified after entry-not-found error"
    );
}

/// `ndoc build <doc.ndoc.typ>` with at least one entry exits 0 and produces a non-empty PDF.
#[test]
fn ndoc_build_ndoc_typ() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("fixture.ndoc.typ");
    let content_file = tmp.path().join("main.typ");
    std::fs::write(&content_file, "Hello, Typst!").expect("write Typst content");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .success();

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "add",
            &abs(&doc),
            "main",
            "--content-file",
            &abs(&content_file),
        ])
        .assert()
        .success();

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&doc)])
        .assert()
        .success();

    let pdf = tmp.path().join("fixture.pdf");
    assert!(pdf.exists(), "ndoc build must produce a PDF at {pdf:?}");
    assert!(
        pdf.metadata().expect("PDF metadata").len() > 0,
        "produced PDF must be non-empty"
    );
}

/// `ndoc build` with an unparseable `.ndoc.typ` file exits non-zero with a human-readable error.
#[test]
fn ndoc_build_malformed_doc() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("bad.ndoc.typ");
    std::fs::write(&doc, "this is not a valid ndoc document\n").expect("write malformed document");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&doc)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

// ---------------------------------------------------------------------------
// E2E tests for `ndoc validate`
// ---------------------------------------------------------------------------

/// `ndoc validate <valid.ndoc.typ>` exits 0 with no violation output.
#[test]
fn validate_valid_ndoc_file() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/valid.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("validate")
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `ndoc validate <invalid.ndoc.typ>` exits 1 and stdout contains the violation location and message.
#[test]
fn validate_invalid_ndoc_file() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/invalid.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("validate")
        .arg(&input)
        .assert()
        .failure()
        .stdout(predicate::str::contains("invalid document header"));
}

/// `ndoc validate <multi_violation.ndoc.typ>` exits 1 and reports all violations, not just the first.
#[test]
fn validate_all_violations_reported() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/multi_violation.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("validate")
        .arg(&input)
        .assert()
        .failure()
        .stdout(predicate::str::contains("alpha"))
        .stdout(predicate::str::contains("beta"));
}

/// `ndoc validate <valid.md>` exits 0.
#[test]
fn validate_valid_md_file() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/valid.md");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("validate")
        .arg(&input)
        .assert()
        .success();
}

/// `ndoc validate <invalid.md>` exits 1 and stdout contains a human-readable violation message.
#[test]
fn validate_invalid_md_file() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/invalid.md");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("validate")
        .arg(&input)
        .assert()
        .failure()
        .stdout(predicate::str::contains("invalid YAML frontmatter"));
}

/// `ndoc validate <file.txt>` exits non-zero and stderr contains "unsupported".
#[test]
fn validate_unsupported_extension() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/unsupported.txt");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("validate")
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported"));
}

// ---------------------------------------------------------------------------
// E2E tests for `ndoc preview`
// ---------------------------------------------------------------------------

/// `ndoc preview <bad.ndoc.typ>` exits non-zero with an actionable error message.
#[test]
fn preview_invalid_input_nonzero() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/invalid.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("preview")
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

/// `ndoc preview <valid.md>` exits 0 when `NDOC_NO_OPEN=1` suppresses the viewer.
#[test]
fn preview_valid_md_exit_zero() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/valid.md");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .env("NDOC_NO_OPEN", "1")
        .arg("preview")
        .arg(&input)
        .assert()
        .success();
}

/// `ndoc preview <compile_failing.md>` exits non-zero with an actionable error message.
#[test]
fn preview_compile_failing_md_nonzero() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/compile_failing.md");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("preview")
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

/// `ndoc preview <file.txt>` exits non-zero with an unsupported-type message.
#[test]
fn preview_unsupported_extension() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/unsupported.txt");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("preview")
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported"));
}

// ---------------------------------------------------------------------------
// T8: Error-path and coverage-gap tests
// ---------------------------------------------------------------------------

/// `ndoc build <file.txt>` exits non-zero with "unsupported" — covers the
/// unsupported-extension bail path in cmd_build.
#[test]
fn e2e_build_unsupported_extension() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/unsupported.txt");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&input)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported"));
}

/// `ndoc preview <valid.ndoc.typ>` exits 0 when `NDOC_NO_OPEN=1` suppresses the
/// viewer — covers the `.ndoc.typ` parse-and-join path in cmd_preview.
#[test]
fn preview_valid_ndoc_typ_exit_zero() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/valid.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .env("NDOC_NO_OPEN", "1")
        .arg("preview")
        .arg(&input)
        .assert()
        .success();
}

/// `ndoc render` dispatches to the scaffold stub and prints a not-yet-implemented
/// message — covers cmd_render and the stub() function body.
#[test]
fn e2e_render_stub_executes() {
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", "placeholder.typ"])
        .assert()
        .success()
        .stdout(predicate::str::contains("scaffolded"));
}

/// `ndoc add` without `--content-file` reads entry content from stdin — covers
/// the stdin branch of read_content.
#[test]
fn ndoc_add_from_stdin() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .success();

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["add", &abs(&doc), "main"])
        .write_stdin("#let content = ()")
        .assert()
        .success();

    let content = std::fs::read_to_string(&doc).expect("read doc after stdin add");
    assert!(
        content.contains("NDOC-ENTRY: main"),
        "entry added via stdin must appear in document"
    );
}
