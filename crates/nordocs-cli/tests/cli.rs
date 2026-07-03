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

/// The `--json` envelope flag on a `doc` subcommand emits a normalized JSON ok envelope.
#[test]
fn doc_json_envelope() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_empty_document(&doc);

    let mut cmd = Command::cargo_bin("ndoc").expect("ndoc binary builds");
    cmd.args(["doc", "outline", "--json", &abs(&doc)])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"ok""#));
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

/// `ndoc build` compiles a canonical composed `.ndoc.typ` (the reference
/// `/*===STATE-START===` format) and resolves its embedded images.
#[test]
fn e2e_build_composed_document_with_embedded_image() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("composed.ndoc.typ");
    write_composed_document(&doc);

    let pdf = tmp.path().join("composed.pdf");
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("build")
        .arg(&doc)
        .assert()
        .success();

    assert!(pdf.exists(), "expected PDF at {pdf:?} was not created");
    assert!(
        pdf.metadata().expect("PDF metadata").len() > 0,
        "PDF at {pdf:?} is empty"
    );
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

/// `ndoc preview <composed.ndoc.typ>` compiles the canonical composed format
/// and exits 0 when `NDOC_NO_OPEN=1` suppresses the viewer.
#[test]
fn preview_composed_document_exit_zero() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("composed.ndoc.typ");
    write_composed_document(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .env("NDOC_NO_OPEN", "1")
        .arg("preview")
        .arg(&doc)
        .assert()
        .success();
}

/// `ndoc validate <composed.ndoc.typ>` exits 0 for a well-formed composed doc.
#[test]
fn validate_composed_document_exit_zero() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("composed.ndoc.typ");
    write_composed_document(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("validate")
        .arg(&doc)
        .assert()
        .success();
}

/// `ndoc validate` on a composed doc whose node uses an unknown component exits
/// non-zero and reports an `[error]` against the built-in catalogue.
#[test]
fn validate_composed_schema_error_exit_nonzero() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("bad.ndoc.typ");
    // `widget` is not in the built-in catalogue under the `default` template.
    let source = "/*===STATE-START===\n\
         {\n  \"templateId\": \"default\",\n  \"themeId\": \"th\",\n  \"nodes\": [\n    { \"id\": \"widget-1\", \"type\": \"widget\" }\n  ]\n}\n\n\
         <document-input>\n---\ntemplateId: default\n---\n</document-input>\n\n\
         <component-input componentId=\"widget\" instance=\"1\">\n---\n---\n</component-input>\n\
         ===STATE-END===*/\n\
         // ===DOCUMENT-START===\n= Doc\n";
    std::fs::write(&doc, source).expect("write fixture");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("validate")
        .arg(&doc)
        .assert()
        .failure()
        .stdout(predicate::str::contains("[error]"))
        .stdout(
            predicate::str::contains("component-not-allowed")
                .or(predicate::str::contains("widget")),
        );
}

/// `ndoc validate` on a composed doc that only triggers a missing-required-input
/// warning still exits 0, printing `[warning]`.
#[test]
fn validate_composed_warning_only_exit_zero() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("warn.ndoc.typ");
    // `heading` is a built-in component requiring `level` and `text`; omitting
    // them yields warnings only (no errors), so validation still passes.
    let source = "/*===STATE-START===\n\
         {\n  \"templateId\": \"default\",\n  \"themeId\": \"th\",\n  \"nodes\": [\n    { \"id\": \"heading-1\", \"type\": \"heading\" }\n  ]\n}\n\n\
         <document-input>\n---\ntemplateId: default\n---\n</document-input>\n\n\
         <component-input componentId=\"heading\" instance=\"1\">\n---\n---\n</component-input>\n\
         ===STATE-END===*/\n\
         // ===DOCUMENT-START===\n= Doc\n";
    std::fs::write(&doc, source).expect("write fixture");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("validate")
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("[warning]"));
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

/// `ndoc render <bare.typ>` exits non-zero: render rejects a bare `.typ`,
/// accepting only the `.ncmp.typ` / `.ndoct.typ` / `.ndoc.typ` suffixes.
#[test]
fn e2e_render_rejects_bare_typ() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("placeholder.typ");
    std::fs::write(&input, "Hello").expect("write bare .typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("placeholder.typ"));
}

/// `ndoc render <fixture.ndoc.typ>` compiles a four-section authoring file to a
/// non-empty PDF at the default suffix-stripped `.pdf` path.
#[test]
fn e2e_render_produces_pdf() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/render.ndoc.typ");
    let expected_pdf = manifest.join("tests/fixtures/render.pdf");

    let _ = std::fs::remove_file(&expected_pdf);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input)])
        .assert()
        .success();

    assert!(
        expected_pdf.exists(),
        "render must create a PDF at {expected_pdf:?}"
    );
    assert!(
        expected_pdf.metadata().expect("PDF metadata").len() > 0,
        "rendered PDF must be non-empty"
    );
    let _ = std::fs::remove_file(&expected_pdf);
}

/// `ndoc render -o <path>` writes the PDF at the override path.
#[test]
fn e2e_render_output_override() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/render.ncmp.typ");
    let tmp = tempfile::tempdir().expect("temp dir");
    let out = tmp.path().join("custom.pdf");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out)])
        .assert()
        .success();

    assert!(
        out.exists(),
        "render -o must write the PDF at the override path"
    );
    assert!(
        out.metadata().expect("PDF metadata").len() > 0,
        "rendered PDF must be non-empty"
    );
}

/// `ndoc render <component>` with no `-o` writes the PDF next to the source,
/// swapping the `.ncmp.typ` suffix for `.pdf`.
#[test]
fn e2e_render_component_default_output_next_to_source() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest.join("tests/fixtures/render.ncmp.typ");
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("widget.ncmp.typ");
    std::fs::copy(&fixture, &input).expect("copy fixture into temp dir");
    let expected_pdf = tmp.path().join("widget.pdf");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input)])
        .assert()
        .success();

    assert!(
        expected_pdf.exists(),
        "render writes the PDF next to the source at {expected_pdf:?}"
    );
    assert!(expected_pdf.metadata().expect("PDF metadata").len() > 0);
}

/// Rendering to the default output path overwrites any pre-existing file there.
#[test]
fn e2e_render_default_output_overwrites_existing_file() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest.join("tests/fixtures/render.ncmp.typ");
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("widget.ncmp.typ");
    std::fs::copy(&fixture, &input).expect("copy fixture into temp dir");
    let out = tmp.path().join("widget.pdf");
    // Pre-seed the default output path with stale, non-PDF content.
    std::fs::write(&out, b"STALE").expect("seed stale file");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input)])
        .assert()
        .success();

    let bytes = std::fs::read(&out).expect("read output");
    assert_eq!(
        &bytes[..5],
        b"%PDF-",
        "the stale file was overwritten with a PDF"
    );
}

/// `ndoc render <missing.ncmp.typ>` exits non-zero and names the offending path.
#[test]
fn e2e_render_missing_input() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let ghost = tmp.path().join("ghost.ncmp.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&ghost)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ghost.ncmp.typ"));
}

/// `ndoc render <fixture> --json` emits `{"status":"ok","data":{"output":...}}`.
#[test]
fn e2e_render_json_envelope() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/render.ncmp.typ");
    let tmp = tempfile::tempdir().expect("temp dir");
    let out = tmp.path().join("rendered.pdf");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "render", &abs(&input), "-o", &abs(&out)])
        .output()
        .expect("command runs");

    assert!(
        output.status.success(),
        "render --json must exit 0 on success"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok", "success envelope must have status 'ok'");
    assert_eq!(
        v["data"]["output"].as_str(),
        Some(abs(&out).as_str()),
        "envelope must report the output path"
    );
}

// ---------------------------------------------------------------------------
// T4: component schema / list subcommands
// ---------------------------------------------------------------------------

/// `ndoc component schema <file>` reports each input's name, kind, and required
/// flag in human-readable form.
#[test]
fn e2e_component_schema() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file = manifest.join("tests/fixtures/component_schema.ncmp.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["component", "schema", &abs(&file)])
        .assert()
        .success()
        .stdout(predicate::str::contains("hero-banner"))
        .stdout(predicate::str::contains("title: string (required)"))
        .stdout(predicate::str::contains("subtitle: content (optional)"));
}

/// `ndoc component schema <file> --json` emits the full schema under `data`.
#[test]
fn e2e_component_schema_json() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file = manifest.join("tests/fixtures/component_schema.ncmp.typ");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "component", "schema", &abs(&file)])
        .output()
        .expect("command runs");

    assert!(output.status.success(), "schema --json must exit 0");
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["data"]["component"]["name"], "hero-banner");
    assert_eq!(v["data"]["component"]["inputs"][0]["name"], "title");
    assert_eq!(v["data"]["component"]["inputs"][0]["kind"], "string");
    assert_eq!(v["data"]["component"]["inputs"][0]["required"], true);
    assert_eq!(v["data"]["component"]["inputs"][1]["required"], false);
}

/// `ndoc component schema <missing>` exits non-zero and names the offending path.
#[test]
fn e2e_component_schema_missing() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let ghost = tmp.path().join("ghost.ncmp.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["component", "schema", &abs(&ghost)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ghost.ncmp.typ"));
}

/// `ndoc component list <dir>` enumerates components in stable (sorted) order.
#[test]
fn e2e_component_list_stable_order() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let body = "/*---\ncomponentId: REPLACE\ninputs:\n  - name: x\n    type: string\n---*/\n";
    std::fs::write(
        tmp.path().join("zeta.ncmp.typ"),
        body.replace("REPLACE", "zeta"),
    )
    .expect("write zeta");
    std::fs::write(
        tmp.path().join("alpha.ncmp.typ"),
        body.replace("REPLACE", "alpha"),
    )
    .expect("write alpha");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "component", "list", &abs(tmp.path())])
        .output()
        .expect("command runs");

    assert!(output.status.success(), "list --json must exit 0");
    let v = parse_json(&output.stdout);
    let names: Vec<&str> = v["data"]["components"]
        .as_array()
        .expect("components array")
        .iter()
        .map(|c| c["name"].as_str().expect("name string"))
        .collect();
    assert_eq!(names, vec!["alpha", "zeta"], "sorted by path");
}

/// `ndoc component list <empty-dir>` reports zero components and exits 0.
#[test]
fn e2e_component_list_empty_dir() {
    let tmp = tempfile::tempdir().expect("temp dir");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["component", "list", &abs(tmp.path())])
        .assert()
        .success()
        .stdout(predicate::str::contains("no components found"));
}

/// `ndoc component list <empty-dir> --json` emits an empty components array.
#[test]
fn e2e_component_list_empty_dir_json() {
    let tmp = tempfile::tempdir().expect("temp dir");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["component", "list", "--json", &abs(tmp.path())])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"ok""#))
        .stdout(predicate::str::contains(r#""components":[]"#));
}

/// `ndoc component list <missing-dir>` exits non-zero with the offending path.
#[test]
fn e2e_component_list_missing_dir() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let ghost = tmp.path().join("nonexistent");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["component", "list", &abs(&ghost)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("nonexistent"));
}

// ---------------------------------------------------------------------------
// T5: template show subcommand
// ---------------------------------------------------------------------------

/// `ndoc template show <path>` reports document inputs and permitted components.
#[test]
fn e2e_template_show() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file = manifest.join("tests/fixtures/template_show.ndoct.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["template", "show", &abs(&file)])
        .assert()
        .success()
        .stdout(predicate::str::contains("fee-proposal"))
        .stdout(predicate::str::contains("title: string (required)"))
        .stdout(predicate::str::contains("date: string (optional)"))
        .stdout(predicate::str::contains("cover-page"))
        .stdout(predicate::str::contains("section-title"));
}

/// `ndoc template show <path> --json` emits the full schema under `data.template`.
#[test]
fn e2e_template_show_json() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file = manifest.join("tests/fixtures/template_show.ndoct.typ");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "template", "show", &abs(&file)])
        .output()
        .expect("command runs");

    assert!(output.status.success(), "show --json must exit 0");
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["data"]["template"]["name"], "fee-proposal");
    assert_eq!(v["data"]["template"]["document_inputs"][0]["name"], "title");
    assert_eq!(
        v["data"]["template"]["document_inputs"][0]["kind"],
        "string"
    );
    assert_eq!(
        v["data"]["template"]["document_inputs"][0]["required"],
        true
    );
    assert_eq!(
        v["data"]["template"]["document_inputs"][1]["required"],
        false
    );
    let allowed: Vec<&str> = v["data"]["template"]["allowed_components"]
        .as_array()
        .expect("allowed_components array")
        .iter()
        .map(|c| c.as_str().expect("component name string"))
        .collect();
    assert_eq!(allowed, vec!["cover-page", "section-title"]);
}

/// `ndoc template show <bare-id>` resolves to `{id}.ndoct.typ` in the cwd.
#[test]
fn e2e_template_show_resolves_bare_id() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let body =
        "/*---\ntemplateId: report\ndocumentInputs:\n  - name: heading\n    type: string\n---*/\n";
    std::fs::write(tmp.path().join("report.ndoct.typ"), body).expect("write template");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .current_dir(tmp.path())
        .args(["template", "show", "report"])
        .assert()
        .success()
        .stdout(predicate::str::contains("report"))
        .stdout(predicate::str::contains("heading: string (required)"));
}

/// `ndoc template show <missing>` exits non-zero and names the offending path.
#[test]
fn e2e_template_show_unknown() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let ghost = tmp.path().join("ghost.ndoct.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["template", "show", &abs(&ghost)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ghost.ndoct.typ"));
}

// ---------------------------------------------------------------------------
// T6: item load / validate subcommands
// ---------------------------------------------------------------------------

/// A component schema (`project`) plus a conforming item, written into `dir`.
///
/// `title` is required; `logo` is an optional image. Used by the item tests to
/// build a self-describing items directory (schema + items side by side).
fn write_project_schema(dir: &Path) {
    let schema = "/*---\n\
        componentId: project\n\
        inputs:\n\
        \x20 - name: title\n\
        \x20   type: string\n\
        \x20 - name: logo\n\
        \x20   type: image\n\
        \x20   required: false\n\
        ---*/\n";
    std::fs::write(dir.join("project.ncmp.typ"), schema).expect("write project schema");
}

/// `ndoc item load <dir>` summarises the collections found and exits 0.
#[test]
fn e2e_item_load() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let item = "---\n$schema: project\n$collection: projects\ntitle: Northwind\n---\n";
    std::fs::write(tmp.path().join("northwind.item.md"), item).expect("write item");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["item", "load", &abs(tmp.path())])
        .assert()
        .success()
        .stdout(predicate::str::contains("projects: 1 items"));
}

/// `ndoc item load <dir> --json` emits a structured collections summary.
#[test]
fn e2e_item_load_json() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let item = "---\n$schema: project\n$collection: projects\ntitle: Northwind\n---\n";
    std::fs::write(tmp.path().join("a.item.md"), item).expect("write item a");
    std::fs::write(tmp.path().join("b.item.md"), item).expect("write item b");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "item", "load", &abs(tmp.path())])
        .output()
        .expect("command runs");

    assert!(output.status.success(), "load --json must exit 0");
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["data"]["collections"][0]["collection"], "projects");
    assert_eq!(v["data"]["collections"][0]["items"], 2);
}

/// `ndoc item validate <dir>` exits 0 when every item conforms to its schema.
#[test]
fn e2e_item_validate_ok() {
    let tmp = tempfile::tempdir().expect("temp dir");
    write_project_schema(tmp.path());
    let item = "---\n$schema: project\n$collection: projects\ntitle: Northwind\n---\n";
    std::fs::write(tmp.path().join("ok.item.md"), item).expect("write item");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["item", "validate", &abs(tmp.path())])
        .assert()
        .success();
}

/// `ndoc item validate <dir>` reports the violation with its source location and
/// exits non-zero when an item is missing a required input.
#[test]
fn e2e_item_validate_fail() {
    let tmp = tempfile::tempdir().expect("temp dir");
    write_project_schema(tmp.path());
    // Missing the required `title` input.
    let item = "---\n$schema: project\n$collection: projects\n---\n";
    std::fs::write(tmp.path().join("broken.item.md"), item).expect("write item");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["item", "validate", &abs(tmp.path())])
        .assert()
        .failure()
        .stdout(predicate::str::contains("broken.item.md"))
        .stdout(predicate::str::contains("missing-input"))
        .stdout(predicate::str::contains("title"));
}

/// `ndoc item validate <dir> --json` emits a `valid` flag plus an issues array.
#[test]
fn e2e_item_validate_json() {
    let tmp = tempfile::tempdir().expect("temp dir");
    write_project_schema(tmp.path());
    let item = "---\n$schema: project\n$collection: projects\n---\n";
    std::fs::write(tmp.path().join("broken.item.md"), item).expect("write item");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "item", "validate", &abs(tmp.path())])
        .output()
        .expect("command runs");

    assert!(
        !output.status.success(),
        "validate with issues must exit non-zero"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["data"]["valid"], false);
    assert_eq!(v["data"]["issues"][0]["code"], "missing-input");
}

/// `ndoc item load <missing-dir>` exits non-zero with the offending path.
#[test]
fn e2e_item_missing_dir() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let ghost = tmp.path().join("nonexistent");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["item", "load", &abs(&ghost)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("nonexistent"));
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

// ---------------------------------------------------------------------------
// T4: --json E2E tests for all subcommands
// ---------------------------------------------------------------------------

/// Parse stdout bytes as a `serde_json::Value`, failing the test if invalid.
fn parse_json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).expect("stdout must be valid JSON when --json is active")
}

/// `ndoc build <file.md> --json` emits a success envelope with `data.output`.
#[test]
fn build_json_success() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let md = tmp.path().join("hello.md");
    std::fs::write(&md, "# Hello\n\nWorld.\n").expect("write markdown fixture");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "build", &abs(&md)])
        .output()
        .expect("command runs");

    assert!(
        output.status.success(),
        "build --json must exit 0 on success"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok", "success envelope must have status 'ok'");
    assert!(
        v["data"]["output"].as_str().is_some(),
        "success envelope must include data.output as a string"
    );
}

/// `ndoc build <missing.md> --json` emits an error envelope with a non-zero exit.
#[test]
fn build_json_failure() {
    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "build", "nonexistent_t4.md"])
        .output()
        .expect("command runs");

    assert!(
        !output.status.success(),
        "build --json must exit non-zero on failure"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(
        v["status"], "error",
        "failure envelope must have status 'error'"
    );
    assert!(
        v["message"].as_str().is_some_and(|m| !m.is_empty()),
        "failure envelope must include a non-empty message"
    );
}

/// `ndoc new <path> --json` emits a success envelope with `data.path`.
#[test]
fn new_json_success() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("fresh.ndoc.typ");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "new", &abs(&doc)])
        .output()
        .expect("command runs");

    assert!(output.status.success(), "new --json must exit 0 on success");
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok", "success envelope must have status 'ok'");
    assert!(
        v["data"]["path"].as_str().is_some(),
        "success envelope must include data.path as a string"
    );
}

/// `ndoc new <existing> --json` emits an error envelope with a non-zero exit.
#[test]
fn new_json_failure() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("existing.ndoc.typ");
    std::fs::write(&doc, "original content").expect("create pre-existing file");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "new", &abs(&doc)])
        .output()
        .expect("command runs");

    assert!(
        !output.status.success(),
        "new --json must exit non-zero when file exists"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(
        v["status"], "error",
        "failure envelope must have status 'error'"
    );
    assert!(
        v["message"].as_str().is_some_and(|m| !m.is_empty()),
        "failure envelope must include a non-empty message"
    );
}

/// `ndoc add <doc> <name> --json` emits a success envelope (data is absent for add).
#[test]
fn add_json_success() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    let content_file = tmp.path().join("comp.typ");
    std::fs::write(&content_file, "#let x = ()").expect("write content file");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .success();

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "--json",
            "add",
            &abs(&doc),
            "mycomp",
            "--content-file",
            &abs(&content_file),
        ])
        .output()
        .expect("command runs");

    assert!(output.status.success(), "add --json must exit 0 on success");
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok", "success envelope must have status 'ok'");
}

/// `ndoc add <missing-doc> <name> --json` emits an error envelope with a non-zero exit.
#[test]
fn add_json_failure() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let ghost = tmp.path().join("ghost.ndoc.typ");
    let content_file = tmp.path().join("comp.typ");
    std::fs::write(&content_file, "#let x = ()").expect("write content file");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "--json",
            "add",
            &abs(&ghost),
            "mycomp",
            "--content-file",
            &abs(&content_file),
        ])
        .output()
        .expect("command runs");

    assert!(
        !output.status.success(),
        "add --json must exit non-zero on failure"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(
        v["status"], "error",
        "failure envelope must have status 'error'"
    );
    assert!(
        v["message"].as_str().is_some_and(|m| !m.is_empty()),
        "failure envelope must include a non-empty message"
    );
}

/// `ndoc edit <doc> <entry> --json` emits a success envelope (data is absent for edit).
#[test]
fn edit_json_success() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    let v1 = tmp.path().join("v1.typ");
    let v2 = tmp.path().join("v2.typ");
    std::fs::write(&v1, "#let x = ()").expect("write v1 content");
    std::fs::write(&v2, "#let x = (updated: true)").expect("write v2 content");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .success();

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["add", &abs(&doc), "mycomp", "--content-file", &abs(&v1)])
        .assert()
        .success();

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "--json",
            "edit",
            &abs(&doc),
            "mycomp",
            "--content-file",
            &abs(&v2),
        ])
        .output()
        .expect("command runs");

    assert!(
        output.status.success(),
        "edit --json must exit 0 on success"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok", "success envelope must have status 'ok'");
}

/// `ndoc edit <doc> <missing-entry> --json` emits an error envelope with a non-zero exit.
#[test]
fn edit_json_failure() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    let content_file = tmp.path().join("content.typ");
    std::fs::write(&content_file, "#let x = ()").expect("write content file");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(&doc)])
        .assert()
        .success();

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "--json",
            "edit",
            &abs(&doc),
            "nonexistent",
            "--content-file",
            &abs(&content_file),
        ])
        .output()
        .expect("command runs");

    assert!(
        !output.status.success(),
        "edit --json must exit non-zero when entry is absent"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(
        v["status"], "error",
        "failure envelope must have status 'error'"
    );
    assert!(
        v["message"].as_str().is_some_and(|m| !m.is_empty()),
        "failure envelope must include a non-empty message"
    );
}

/// `ndoc validate <valid.ndoc.typ> --json` emits status 'ok' with an empty violations list.
#[test]
fn validate_json_valid() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/valid.ndoc.typ");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "validate", &abs(&input)])
        .output()
        .expect("command runs");

    assert!(
        output.status.success(),
        "validate --json must exit 0 for a valid file"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok", "success envelope must have status 'ok'");
    let violations = v["data"]["violations"]
        .as_array()
        .expect("data.violations must be an array");
    assert!(
        violations.is_empty(),
        "valid file must produce an empty violations list"
    );
}

/// `ndoc validate <invalid.ndoc.typ> --json` emits status 'ok' with a non-empty violations list
/// and exits non-zero (violations are reported through `data`, not the error envelope).
#[test]
fn validate_json_violations() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/invalid.ndoc.typ");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "validate", &abs(&input)])
        .output()
        .expect("command runs");

    assert!(
        !output.status.success(),
        "validate --json must exit non-zero when violations exist"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(
        v["status"], "ok",
        "violations are reported in data, not as an error envelope"
    );
    let violations = v["data"]["violations"]
        .as_array()
        .expect("data.violations must be an array");
    assert!(
        !violations.is_empty(),
        "invalid file must produce at least one violation"
    );
    assert!(
        violations[0]["message"].as_str().is_some(),
        "each violation must have a message string"
    );
}

/// `ndoc validate <missing.ndoc.typ> --json` emits an error envelope with a non-zero exit.
#[test]
fn validate_json_failure() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let ghost = tmp.path().join("ghost.ndoc.typ");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "validate", &abs(&ghost)])
        .output()
        .expect("command runs");

    assert!(
        !output.status.success(),
        "validate --json must exit non-zero for an unreadable file"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(
        v["status"], "error",
        "failure envelope must have status 'error'"
    );
    assert!(
        v["message"].as_str().is_some_and(|m| !m.is_empty()),
        "failure envelope must include a non-empty message"
    );
}

/// `ndoc preview <valid.md> --json` emits a success envelope with `data.preview_path`.
#[test]
fn preview_json_success() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let md = tmp.path().join("preview.md");
    std::fs::write(&md, "# Preview\n\nA simple document.\n").expect("write markdown fixture");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "preview", &abs(&md)])
        .output()
        .expect("command runs");

    assert!(
        output.status.success(),
        "preview --json must exit 0 on success"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok", "success envelope must have status 'ok'");
    assert!(
        v["data"]["preview_path"].as_str().is_some(),
        "success envelope must include data.preview_path as a string"
    );
}

/// `ndoc preview <invalid.ndoc.typ> --json` emits an error envelope with a non-zero exit.
#[test]
fn preview_json_failure() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/invalid.ndoc.typ");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "preview", &abs(&input)])
        .output()
        .expect("command runs");

    assert!(
        !output.status.success(),
        "preview --json must exit non-zero for an invalid input"
    );
    let v = parse_json(&output.stdout);
    assert_eq!(
        v["status"], "error",
        "failure envelope must have status 'error'"
    );
    assert!(
        v["message"].as_str().is_some_and(|m| !m.is_empty()),
        "failure envelope must include a non-empty message"
    );
}

// ---------------------------------------------------------------------------
// T6: Release binary smoke test (gated with #[ignore])
// ---------------------------------------------------------------------------

/// Release-binary smoke test: locates `target/release/ndoc` built from the
/// current workspace and verifies that `ndoc build tests/fixtures/sample.md`
/// exits 0 with a non-empty PDF.
///
/// This test is skipped by default; run it explicitly after `cargo build --release`:
///
/// ```sh
/// cargo build --release
/// cargo test -- --ignored release_smoke_test
/// ```
#[test]
#[ignore]
fn release_smoke_test() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release_binary = manifest.join("target/release/ndoc");
    let input = manifest.join("tests/fixtures/sample.md");
    let expected_pdf = manifest.join("tests/fixtures/sample.pdf");

    // Remove any artifact from a previous run so we can assert this run created it.
    let _ = std::fs::remove_file(&expected_pdf);

    let status = std::process::Command::new(&release_binary)
        .arg("build")
        .arg(&input)
        .status()
        .unwrap_or_else(|e| {
            panic!(
                "release binary at {release_binary:?} could not be executed: {e}\n\
                 Run `cargo build --release` before this test."
            )
        });

    assert!(
        status.success(),
        "release `ndoc build` must exit 0 for a valid Markdown input"
    );
    assert!(
        expected_pdf.exists(),
        "release `ndoc build` must create a PDF at {expected_pdf:?}"
    );
    assert!(
        expected_pdf.metadata().expect("PDF metadata").len() > 0,
        "release `ndoc build` must produce a non-empty PDF"
    );
}

// ---------------------------------------------------------------------------
// T7: image add subcommand
// ---------------------------------------------------------------------------

/// Seed a minimal canonical composed `.ndoc.typ` document (the
/// `/*===STATE-START===` reference format) with one embedded image at `path`.
fn write_composed_document(path: &Path) {
    use base64::Engine as _;

    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#0a0"/></svg>"##;
    let b64 = base64::engine::general_purpose::STANDARD.encode(svg.as_bytes());
    // `default` is a built-in template; with no nodes the document validates clean.
    let source = format!(
        "/*===STATE-START===\n\
         {{\n  \"templateId\": \"default\",\n  \"themeId\": \"th\",\n  \"nodes\": [],\n  \"images\": [\n    {{ \"name\": \"logo.svg\", \"hash\": \"abc123\" }}\n  ]\n}}\n\n\
         <document-input>\n---\ntemplateId: default\n---\n</document-input>\n\
         ===STATE-END===*/\n\
         /*===IMAGES-START===\n---abc123---\n{b64}\n---END---\n===IMAGES-END===*/\n\
         // ===TEMPLATE-START===\n// ===TEMPLATE-END===\n\
         // ===DOCUMENT-START===\n= Composed\n#image(\"images/logo.svg\", width: 10pt)\n"
    );
    std::fs::write(path, source).expect("seed composed document");
}

/// Seed an empty four-section `.ndoc.typ` document at `path`.
fn write_empty_document(path: &Path) {
    let doc = nordocs_core::model::Document {
        template: "article".to_string(),
        inputs: std::collections::BTreeMap::new(),
        nodes: Vec::new(),
        images: Vec::new(),
    };
    nordocs_core::authoring::doc_state::write_document(path, &doc).expect("seed document");
}

/// `ndoc image add` records the image in the manifest and exits 0.
#[test]
fn e2e_image_add() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    let img = tmp.path().join("logo.png");
    write_empty_document(&doc);
    std::fs::write(&img, b"PNGBYTES").expect("write image");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["image", "add", &abs(&doc), &abs(&img)])
        .assert()
        .success()
        .stdout(predicate::str::contains("logo.png"));

    let back = nordocs_core::authoring::doc_state::read_document(&doc).expect("read back");
    assert_eq!(back.images.len(), 1);
    assert_eq!(back.images[0].name, "logo.png");
}

/// Re-embedding identical content is idempotent (no duplicate manifest entry).
#[test]
fn e2e_image_add_idempotent() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    let img = tmp.path().join("logo.png");
    write_empty_document(&doc);
    std::fs::write(&img, b"SAMECONTENT").expect("write image");

    for _ in 0..2 {
        Command::cargo_bin("ndoc")
            .expect("ndoc binary builds")
            .args(["image", "add", &abs(&doc), &abs(&img)])
            .assert()
            .success();
    }

    let back = nordocs_core::authoring::doc_state::read_document(&doc).expect("read back");
    assert_eq!(
        back.images.len(),
        1,
        "re-embedding identical content must not duplicate the manifest entry"
    );
}

/// `--json` emits the ok envelope carrying the embedded image's name and hash.
#[test]
fn e2e_image_add_json_envelope() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    let img = tmp.path().join("logo.png");
    write_empty_document(&doc);
    std::fs::write(&img, b"PNGBYTES").expect("write image");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["image", "add", "--json", &abs(&doc), &abs(&img)])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"ok""#))
        .stdout(predicate::str::contains(r#""name":"logo.png""#));
}

/// A non-`.ndoc.typ` target exits non-zero with a clear message.
#[test]
fn e2e_image_add_unsupported_target() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let target = tmp.path().join("component.ncmp.typ");
    let img = tmp.path().join("logo.png");
    std::fs::write(&target, "/*---\ncomponentId: x\n---*/\n").expect("write target");
    std::fs::write(&img, b"PNGBYTES").expect("write image");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["image", "add", &abs(&target), &abs(&img)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported target"));
}

/// A missing image file exits non-zero with the offending path named.
#[test]
fn e2e_image_add_missing_image() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_empty_document(&doc);
    let missing = tmp.path().join("nope.png");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["image", "add", &abs(&doc), &abs(&missing)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("nope.png"));
}

/// Path to the bundled template fixture used by `doc new` tests.
fn template_fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/template_show.ndoct.typ")
}

/// `ndoc doc new <template> -o <path>` creates a template-bound document.
#[test]
fn e2e_doc_new() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let out = tmp.path().join("proposal.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "new", &abs(&template_fixture()), "-o", &abs(&out)])
        .assert()
        .success();

    let doc = nordocs_core::authoring::doc_state::read_document(&out).expect("read created doc");
    assert_eq!(
        doc.template, "fee-proposal",
        "document is bound to template"
    );
    assert!(doc.nodes.is_empty(), "new document starts empty");
}

/// `--json` reports the created path in the ok envelope.
#[test]
fn e2e_doc_new_json_envelope() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let out = tmp.path().join("proposal.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "new",
            "--json",
            &abs(&template_fixture()),
            "-o",
            &abs(&out),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"ok""#))
        .stdout(predicate::str::contains("proposal.ndoc.typ"));
}

/// `doc new` refuses to overwrite an existing output path.
#[test]
fn e2e_doc_new_refuses_overwrite() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let out = tmp.path().join("exists.ndoc.typ");
    std::fs::write(&out, "// pre-existing").expect("seed existing file");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "new", &abs(&template_fixture()), "-o", &abs(&out)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to overwrite"));
}

/// An unknown template id exits non-zero with the offending path named.
#[test]
fn e2e_doc_new_unknown_template() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let out = tmp.path().join("out.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "new", "nonexistent-template", "-o", &abs(&out)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("nonexistent-template.ndoct.typ"));
}

/// Seed a four-section document with a nested node tree for outline tests.
fn write_document_with_nodes(path: &Path) {
    use nordocs_core::model::{Document, Node, NodeId};
    let doc = Document {
        template: "fee-proposal".to_string(),
        inputs: std::collections::BTreeMap::new(),
        nodes: vec![Node {
            id: NodeId("section-aabb".to_string()),
            component: "section".to_string(),
            inputs: std::collections::BTreeMap::new(),
            children: vec![Node {
                id: NodeId("para-0001".to_string()),
                component: "paragraph".to_string(),
                inputs: std::collections::BTreeMap::new(),
                children: Vec::new(),
            }],
        }],
        images: Vec::new(),
    };
    nordocs_core::authoring::doc_state::write_document(path, &doc).expect("seed document");
}

/// `ndoc doc outline` prints node ids, component types, and nesting in order.
#[test]
fn e2e_doc_outline() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_nodes(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "outline", &abs(&doc)])
        .assert()
        .success()
        .stdout(predicate::str::contains("section-aabb (section)"))
        .stdout(predicate::str::contains("  para-0001 (paragraph)"));
}

/// `--json` emits a structured node tree (id + component + nested children).
#[test]
fn e2e_doc_outline_json_tree() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_nodes(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "outline", "--json", &abs(&doc)])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""id":"section-aabb""#))
        .stdout(predicate::str::contains(r#""component":"section""#))
        .stdout(predicate::str::contains(r#""id":"para-0001""#));
}

/// The human-readable outline rendering is frozen so the addressing format
/// (stable id + component + two-space-per-depth nesting) cannot drift silently.
#[test]
fn snapshot_doc_outline() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_nodes(&doc);

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "outline", &abs(&doc)])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let rendered = String::from_utf8(output).expect("outline output is utf-8");
    insta::assert_snapshot!(rendered);
}

/// A missing document exits non-zero with the offending path named.
#[test]
fn e2e_doc_outline_missing() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let missing = tmp.path().join("nope.ndoc.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "outline", &abs(&missing)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("nope.ndoc.typ"));
}

/// `ndoc doc add --type <component>` mints a node at the root and reports its id.
#[test]
fn e2e_doc_add_at_root() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_empty_document(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "add", &abs(&doc), "--type", "heading"])
        .assert()
        .success()
        .stdout(predicate::str::contains("heading-"));

    let read = nordocs_core::authoring::doc_state::read_document(&doc).expect("read back");
    assert_eq!(read.nodes.len(), 1, "one root node was added");
    assert_eq!(read.nodes[0].component, "heading");
}

/// `--json` reports the freshly minted node id in the ok envelope.
#[test]
fn e2e_doc_add_json_envelope() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_empty_document(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "add", "--json", &abs(&doc), "--type", "paragraph"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"ok""#))
        .stdout(predicate::str::contains(r#""node_id":"paragraph-"#));
}

/// `--parent <id>` nests the new node under an existing node; `--inputs` seeds it.
#[test]
fn e2e_doc_add_under_parent_with_inputs() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_empty_document(&doc);

    // Seed a root node, then nest a child under it.
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "add", &abs(&doc), "--type", "heading"])
        .assert()
        .success();
    let parent_id = nordocs_core::authoring::doc_state::read_document(&doc)
        .expect("read back")
        .nodes[0]
        .id
        .0
        .clone();

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "add",
            &abs(&doc),
            "--type",
            "paragraph",
            "--parent",
            &parent_id,
            "--inputs",
            "text=hello",
        ])
        .assert()
        .success();

    let read = nordocs_core::authoring::doc_state::read_document(&doc).expect("read back");
    assert_eq!(read.nodes.len(), 1, "still one root node");
    let child = &read.nodes[0].children[0];
    assert_eq!(child.component, "paragraph");
    assert_eq!(
        child.inputs.get("text").map(|v| &v.value),
        Some(&serde_json::Value::String("hello".to_string())),
        "seed input is stored"
    );
}

/// An unknown component type leaves the document unchanged and exits non-zero.
#[test]
fn e2e_doc_add_unknown_type() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_empty_document(&doc);
    let before = std::fs::read_to_string(&doc).expect("read before");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "add", &abs(&doc), "--type", "bogus-component"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown component type"));

    let after = std::fs::read_to_string(&doc).expect("read after");
    assert_eq!(before, after, "document is unchanged on error");
}

/// `--before` and `--after` insert siblings on the correct side of a target.
#[test]
fn e2e_doc_add_sibling_placement() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_nodes(&doc);

    // Tree starts as root [section-aabb]. Insert a heading after it...
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "add",
            &abs(&doc),
            "--type",
            "heading",
            "--after",
            "section-aabb",
        ])
        .assert()
        .success();
    // ...and a paragraph before it.
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "add",
            &abs(&doc),
            "--type",
            "paragraph",
            "--before",
            "section-aabb",
        ])
        .assert()
        .success();

    let read = nordocs_core::authoring::doc_state::read_document(&doc).expect("read back");
    let order: Vec<&str> = read.nodes.iter().map(|n| n.component.as_str()).collect();
    assert_eq!(
        order,
        vec!["paragraph", "section", "heading"],
        "before/after place siblings on the expected side of the target"
    );
}

/// An unknown placement target leaves the document unchanged and exits non-zero.
#[test]
fn e2e_doc_add_unknown_parent() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_empty_document(&doc);
    let before = std::fs::read_to_string(&doc).expect("read before");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "add",
            &abs(&doc),
            "--type",
            "heading",
            "--parent",
            "missing-1234",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown parent node id"));

    let after = std::fs::read_to_string(&doc).expect("read after");
    assert_eq!(before, after, "document is unchanged on error");
}

/// An unknown `--before` sibling id exits non-zero and leaves the document
/// unchanged (parity with the reference SiblingNotFound outcome).
#[test]
fn e2e_doc_add_unknown_sibling() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_empty_document(&doc);
    let before = std::fs::read_to_string(&doc).expect("read before");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "add",
            &abs(&doc),
            "--type",
            "heading",
            "--before",
            "missing-1234",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown node id"));

    let after = std::fs::read_to_string(&doc).expect("read after");
    assert_eq!(
        before, after,
        "document is unchanged on a missing sibling target"
    );
}

/// `--before` and `--after` are mutually exclusive placements: supplying both is
/// rejected with a usage error and a non-zero exit (parity with the reference
/// ConflictingPosition outcome).
#[test]
fn e2e_doc_add_conflicting_placement() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_nodes(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "add",
            &abs(&doc),
            "--type",
            "heading",
            "--before",
            "section-aabb",
            "--after",
            "section-aabb",
        ])
        .assert()
        .failure();
}

/// Adding a node with an empty image-typed input stores the empty value without
/// embedding anything: the image manifest is untouched (parity with the
/// reference AddNode_WithEmptyImageInput_DoesNotTouchImageState outcome).
#[test]
fn e2e_doc_add_empty_image_input_leaves_manifest_untouched() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_empty_document(&doc);
    let images_before = nordocs_core::authoring::doc_state::read_document(&doc)
        .expect("read before")
        .images
        .len();

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "add",
            &abs(&doc),
            "--type",
            "paragraph",
            "--inputs",
            "image=",
        ])
        .assert()
        .success();

    let after = nordocs_core::authoring::doc_state::read_document(&doc).expect("read after");
    assert_eq!(
        after.images.len(),
        images_before,
        "an empty image input must not add any manifest entry"
    );
}

/// `ndoc doc remove <id>` (default) drops the node but promotes its children.
#[test]
fn e2e_doc_remove_preserves_children() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_nodes(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "remove", &abs(&doc), "section-aabb"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed section-aabb"));

    let read = nordocs_core::authoring::doc_state::read_document(&doc).expect("read back");
    assert_eq!(read.nodes.len(), 1, "child was promoted to root");
    assert_eq!(
        read.nodes[0].id.0, "para-0001",
        "the preserved child takes the removed node's place"
    );
}

/// `--with-children` drops the node and its whole subtree.
#[test]
fn e2e_doc_remove_with_children() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_nodes(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "remove",
            &abs(&doc),
            "section-aabb",
            "--with-children",
        ])
        .assert()
        .success();

    let read = nordocs_core::authoring::doc_state::read_document(&doc).expect("read back");
    assert!(read.nodes.is_empty(), "whole subtree was dropped");
}

/// An unknown node id leaves the document unchanged and exits non-zero.
#[test]
fn e2e_doc_remove_unknown() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_nodes(&doc);
    let before = std::fs::read_to_string(&doc).expect("read before");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "remove", &abs(&doc), "ghost-9999"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown node id"));

    let after = std::fs::read_to_string(&doc).expect("read after");
    assert_eq!(before, after, "document is unchanged on error");
}

/// Seed a document with a single built-in `heading` node (`level: number`,
/// `text: content`) so `doc set` can validate against a real component schema.
fn write_document_with_heading(path: &Path) {
    use nordocs_core::model::{Document, Node, NodeId};
    let doc = Document {
        template: "default".to_string(),
        inputs: std::collections::BTreeMap::new(),
        nodes: vec![Node {
            id: NodeId("heading-1234".to_string()),
            component: "heading".to_string(),
            inputs: std::collections::BTreeMap::new(),
            children: Vec::new(),
        }],
        images: Vec::new(),
    };
    nordocs_core::authoring::doc_state::write_document(path, &doc).expect("seed document");
}

/// `ndoc doc set <node> --key --value` coerces the value to the declared kind.
#[test]
fn e2e_doc_set_node_input() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_heading(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "set",
            &abs(&doc),
            "heading-1234",
            "--key",
            "level",
            "--value",
            "2",
        ])
        .assert()
        .success();

    let read = nordocs_core::authoring::doc_state::read_document(&doc).expect("read back");
    let input = &read.nodes[0].inputs["level"];
    assert_eq!(input.kind, nordocs_core::model::InputKind::Number);
    assert_eq!(input.value, serde_json::json!(2.0));
}

/// `--document` targets a document-level input validated against the template.
#[test]
fn e2e_doc_set_document_input() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_heading(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "set",
            &abs(&doc),
            "--document",
            "--key",
            "title",
            "--value",
            "My Doc",
        ])
        .assert()
        .success();

    let read = nordocs_core::authoring::doc_state::read_document(&doc).expect("read back");
    assert_eq!(read.inputs["title"].value, serde_json::json!("My Doc"));
}

/// `--json` emits the ok envelope carrying the target, key, and value.
#[test]
fn e2e_doc_set_json_envelope() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_heading(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "set",
            "--json",
            &abs(&doc),
            "heading-1234",
            "--key",
            "text",
            "--value",
            "Hello",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"ok""#))
        .stdout(predicate::str::contains(r#""key":"text""#));
}

/// A value that does not match the declared kind leaves the document unchanged.
#[test]
fn e2e_doc_set_kind_mismatch() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_heading(&doc);
    let before = std::fs::read_to_string(&doc).expect("read before");

    // `level` is a number; a non-numeric value must be rejected.
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "set",
            &abs(&doc),
            "heading-1234",
            "--key",
            "level",
            "--value",
            "not-a-number",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a valid number"));

    let after = std::fs::read_to_string(&doc).expect("read after");
    assert_eq!(before, after, "document is unchanged on error");
}

/// An input key absent from the schema leaves the document unchanged.
#[test]
fn e2e_doc_set_unknown_key() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_heading(&doc);
    let before = std::fs::read_to_string(&doc).expect("read before");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "set",
            &abs(&doc),
            "heading-1234",
            "--key",
            "nonsense",
            "--value",
            "x",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not in the schema"));

    let after = std::fs::read_to_string(&doc).expect("read after");
    assert_eq!(before, after, "document is unchanged on error");
}

/// An unknown node id leaves the document unchanged and exits non-zero.
#[test]
fn e2e_doc_set_unknown_node() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_heading(&doc);
    let before = std::fs::read_to_string(&doc).expect("read before");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "set",
            &abs(&doc),
            "ghost-9999",
            "--key",
            "text",
            "--value",
            "x",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown node id"));

    let after = std::fs::read_to_string(&doc).expect("read after");
    assert_eq!(before, after, "document is unchanged on error");
}

/// Choosing both a node id and `--document` (or neither) is rejected.
#[test]
fn e2e_doc_set_requires_one_target() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("doc.ndoc.typ");
    write_document_with_heading(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "doc",
            "set",
            &abs(&doc),
            "heading-1234",
            "--document",
            "--key",
            "text",
            "--value",
            "x",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exactly one target"));
}

/// `ndoc doc schema <component>` reports the component's declared inputs.
#[test]
fn e2e_doc_schema_component() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/component_schema.ncmp.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "schema", &abs(&fixture)])
        .assert()
        .success()
        .stdout(predicate::str::contains("component: hero-banner"))
        .stdout(predicate::str::contains("title: string (required)"))
        .stdout(predicate::str::contains("subtitle: content (optional)"));
}

/// `ndoc doc schema <template>` reports the template's document inputs (JSON).
#[test]
fn e2e_doc_schema_template() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/template_show.ndoct.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "schema", "--json", &abs(&fixture)])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"ok""#))
        .stdout(predicate::str::contains(r#""name":"fee-proposal""#));
}

/// An unreadable schema target exits non-zero with the offending path named.
#[test]
fn e2e_doc_schema_missing() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let missing = tmp.path().join("nope.ncmp.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["doc", "schema", &abs(&missing)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("nope.ncmp.typ"));
}

/// No in-scope command may print the legacy "scaffolded but not yet implemented"
/// message, on either the success or the failure path. Each command is invoked
/// so its body actually runs (a bare path argument that does not exist drives
/// every command into a real error branch rather than an arg-parse error), and
/// the scaffold phrase is asserted absent from both stdout and stderr. This is a
/// behavioural regression guard against any command being re-stubbed.
#[test]
fn no_scaffold_message_anywhere() {
    const SCAFFOLD: &str = "scaffolded but not yet implemented";

    // Every in-scope command group, each driven into its command body with a
    // single non-existent path/argument that forces a real (non-arg-parse) path.
    let invocations: &[&[&str]] = &[
        &["render", "missing.ndoc.typ"],
        &["component", "schema", "missing.ncmp.typ"],
        &["component", "list", "missing-dir"],
        &["template", "show", "missing.ndoct.typ"],
        &["item", "load", "missing-dir"],
        &["item", "validate", "missing-dir"],
        &["image", "add", "missing.ndoc.typ", "missing.png"],
        &["doc", "new", "missing.ndoct.typ"],
        &["doc", "outline", "missing.ndoc.typ"],
        &["doc", "add", "missing.ndoc.typ", "--type", "heading"],
        &["doc", "remove", "missing.ndoc.typ", "heading-0000"],
        &[
            "doc",
            "set",
            "missing.ndoc.typ",
            "heading-0000",
            "--key",
            "k",
            "--value",
            "v",
        ],
        &["doc", "schema", "missing.ncmp.typ"],
    ];

    for argv in invocations {
        let output = Command::cargo_bin("ndoc")
            .expect("ndoc binary builds")
            .args(*argv)
            .output()
            .expect("command runs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stdout.contains(SCAFFOLD) && !stderr.contains(SCAFFOLD),
            "command {argv:?} emitted the scaffold message:\nstdout: {stdout}\nstderr: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------
// multi-format-export: render/build to SVG and PNG, --format/--dpi/--merged,
// multi-page naming, and the -o/--format conflict error path.
// ---------------------------------------------------------------------------

/// A single-page render-accepted source (`.ncmp.typ` so `render` accepts it).
const ONE_PAGE_TYP: &str = "#set page(width: 120pt, height: 80pt, margin: 10pt)\nPage one";

/// A two-page render-accepted source separated by an explicit page break.
const TWO_PAGE_TYP: &str =
    "#set page(width: 120pt, height: 80pt, margin: 10pt)\nPage one\n#pagebreak()\nPage two";

/// Read a PNG's big-endian IHDR width (bytes 16..20). Panics if not a PNG.
fn png_width(bytes: &[u8]) -> u32 {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "expected PNG magic");
    u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]])
}

/// `ndoc render -o out.svg` infers SVG from the extension and writes a real SVG.
#[test]
fn e2e_render_svg_by_output_extension() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let out = tmp.path().join("page.svg");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out)])
        .assert()
        .success();

    let svg = std::fs::read_to_string(&out).expect("read SVG");
    assert!(
        svg.starts_with("<svg"),
        "expected an SVG document: {svg:.40}"
    );
    assert!(svg.contains("</svg>"), "expected a complete SVG document");
}

/// `ndoc render -o out.png` infers PNG from the extension and writes a real PNG.
#[test]
fn e2e_render_png_by_output_extension() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let out = tmp.path().join("page.png");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out)])
        .assert()
        .success();

    let png = std::fs::read(&out).expect("read PNG");
    assert!(png_width(&png) > 0, "decoded PNG width must be positive");
}

/// `ndoc render --format svg` (no `-o`) writes `<base>.svg` next to the source.
#[test]
fn e2e_render_format_flag_default_path() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let expected = tmp.path().join("page.svg");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "--format", "svg"])
        .assert()
        .success();

    let svg = std::fs::read_to_string(&expected).expect("read default SVG path");
    assert!(svg.starts_with("<svg"), "expected an SVG document");
}

/// `--dpi` scales the rasterised PNG: a higher DPI yields a wider image.
#[test]
fn e2e_render_dpi_scales_png() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let low = tmp.path().join("low.png");
    let high = tmp.path().join("high.png");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&low), "--dpi", "72"])
        .assert()
        .success();
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&high), "--dpi", "300"])
        .assert()
        .success();

    let low_w = png_width(&std::fs::read(&low).expect("read low png"));
    let high_w = png_width(&std::fs::read(&high).expect("read high png"));
    assert!(
        high_w > low_w,
        "higher DPI must widen the PNG ({high_w} !> {low_w})"
    );
}

/// A multi-page document without `--merged` writes `<base>-1.svg`/`<base>-2.svg`
/// and never the bare `<base>.svg`.
#[test]
fn e2e_render_multipage_naming() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("doc.ncmp.typ");
    std::fs::write(&input, TWO_PAGE_TYP).expect("write input");
    let out = tmp.path().join("doc.svg");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out)])
        .assert()
        .success();

    let page1 = tmp.path().join("doc-1.svg");
    let page2 = tmp.path().join("doc-2.svg");
    assert!(page1.exists(), "page 1 SVG must exist at {page1:?}");
    assert!(page2.exists(), "page 2 SVG must exist at {page2:?}");
    assert!(
        !out.exists(),
        "the bare base path must not be written for a multi-page split"
    );
    let svg1 = std::fs::read_to_string(&page1).expect("read page 1");
    assert!(svg1.starts_with("<svg"), "page 1 must be an SVG");
}

/// `--merged` writes one `<base>.svg` for a multi-page document.
#[test]
fn e2e_render_merged_svg() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("doc.ncmp.typ");
    std::fs::write(&input, TWO_PAGE_TYP).expect("write input");
    let out = tmp.path().join("merged.svg");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out), "--merged"])
        .assert()
        .success();

    assert!(out.exists(), "merged SVG must be a single file at {out:?}");
    assert!(
        !tmp.path().join("merged-1.svg").exists(),
        "merged output must not produce per-page files"
    );
    let svg = std::fs::read_to_string(&out).expect("read merged SVG");
    assert!(svg.starts_with("<svg"), "merged output must be an SVG");
}

/// A single-page document writes the bare `<base>.png` (no `-1` suffix).
#[test]
fn e2e_render_single_page_no_suffix() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let out = tmp.path().join("page.png");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out)])
        .assert()
        .success();

    assert!(out.exists(), "single page writes the bare base path");
    assert!(
        !tmp.path().join("page-1.png").exists(),
        "a single-page document must not get a -1 suffix"
    );
}

/// `-o out.svg --format png` is a hard error mentioning the conflict.
#[test]
fn e2e_render_format_conflict_errors() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let out = tmp.path().join("page.svg");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out), "--format", "png"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("conflict"));
    assert!(
        !out.exists(),
        "no output is written when the formats conflict"
    );
}

/// `ndoc render --json -o out.svg` reports the written paths under `outputs`.
#[test]
fn e2e_render_svg_json_reports_outputs() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let out = tmp.path().join("page.svg");

    let output = Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["--json", "render", &abs(&input), "-o", &abs(&out)])
        .output()
        .expect("command runs");

    assert!(output.status.success(), "render --json must exit 0");
    let v = parse_json(&output.stdout);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["data"]["outputs"][0].as_str(), Some(abs(&out).as_str()));
    assert_eq!(v["data"]["output"].as_str(), Some(abs(&out).as_str()));
}

/// `ndoc build <file.md> --format svg` writes `<base>.svg`.
#[test]
fn e2e_build_svg() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let md = tmp.path().join("doc.md");
    std::fs::write(&md, "# Heading\n\nBody text.\n").expect("write markdown");
    let expected = tmp.path().join("doc.svg");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&md), "--format", "svg"])
        .assert()
        .success();

    let svg = std::fs::read_to_string(&expected).expect("read built SVG");
    assert!(svg.starts_with("<svg"), "build must emit an SVG document");
}

/// `ndoc build <file.md> --format png` writes a real PNG at `<base>.png`.
#[test]
fn e2e_build_png() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let md = tmp.path().join("doc.md");
    std::fs::write(&md, "# Heading\n\nBody text.\n").expect("write markdown");
    let expected = tmp.path().join("doc.png");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&md), "--format", "png"])
        .assert()
        .success();

    let png = std::fs::read(&expected).expect("read built PNG");
    assert!(png_width(&png) > 0, "build PNG must have positive width");
}

/// `ndoc build <file.md> --format pdf` still writes a PDF (default path).
#[test]
fn e2e_build_pdf_format_flag() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let md = tmp.path().join("doc.md");
    std::fs::write(&md, "# Heading\n\nBody text.\n").expect("write markdown");
    let expected = tmp.path().join("doc.pdf");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&md), "--format", "pdf"])
        .assert()
        .success();

    let pdf = std::fs::read(&expected).expect("read built PDF");
    assert_eq!(&pdf[..5], b"%PDF-", "build --format pdf must emit a PDF");
}

// ---------------------------------------------------------------------------
// E2E tests for the hidden `ndoc jump` source-mapping diagnostic
// ---------------------------------------------------------------------------

/// `ndoc jump <fixture> --page 1 --at <x>,<y> --json` resolves a click on the
/// rendered glyph back to the expected source file/line/column.
#[test]
fn e2e_jump_resolves_source_location_json() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/jump.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "jump",
            &abs(&input),
            "--page",
            "1",
            "--at",
            "13,14",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"ok""#))
        .stdout(predicate::str::contains(r#""kind":"file""#))
        .stdout(predicate::str::contains(r#""path":"main.typ""#))
        .stdout(predicate::str::contains(r#""line":2"#))
        .stdout(predicate::str::contains(r#""column":2"#));
}

/// A click on empty space yields an `ok` envelope carrying a null jump target.
#[test]
fn e2e_jump_empty_space_is_null_json() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/jump.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args([
            "jump",
            &abs(&input),
            "--page",
            "1",
            "--at",
            "100,70",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"ok""#))
        .stdout(predicate::str::contains(r#""data":null"#));
}

/// `ndoc jump` is hidden from the top-level help but `ndoc jump --help` works.
#[test]
fn e2e_jump_hidden_from_top_level_help() {
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("jump").not());

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["jump", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--at"));
}

/// Failure path: a malformed `--at` coordinate exits non-zero with guidance.
#[test]
fn e2e_jump_bad_coordinate_fails() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/jump.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["jump", &abs(&input), "--page", "1", "--at", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--at must be in the form"));
}

// ---------------------------------------------------------------------------
// multi-format-export hardening: additional success/failure coverage for the
// formats and flags not already exercised above (render PDF/PNG via flag,
// merged & multi-page PNG, build --dpi/--merged/multi-page, agreement vs
// conflict on -o/--format, and invalid flag-value failure paths).
// ---------------------------------------------------------------------------

/// `ndoc render --format pdf` (no `-o`) writes `<base>.pdf` and emits a PDF.
#[test]
fn e2e_render_format_pdf_default_path() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let expected = tmp.path().join("page.pdf");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "--format", "pdf"])
        .assert()
        .success();

    let pdf = std::fs::read(&expected).expect("read default PDF path");
    assert_eq!(&pdf[..5], b"%PDF-", "render --format pdf must emit a PDF");
}

/// `ndoc render --format png` (no `-o`) writes `<base>.png` next to the source.
#[test]
fn e2e_render_format_png_default_path() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let expected = tmp.path().join("page.png");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "--format", "png"])
        .assert()
        .success();

    let png = std::fs::read(&expected).expect("read default PNG path");
    assert!(png_width(&png) > 0, "decoded PNG width must be positive");
}

/// A recognised `-o` extension that AGREES with `--format` is accepted (the
/// success counterpart to the conflict failure path).
#[test]
fn e2e_render_output_and_format_agree() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let out = tmp.path().join("agree.png");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out), "--format", "png"])
        .assert()
        .success();

    let png = std::fs::read(&out).expect("read PNG");
    assert!(
        png_width(&png) > 0,
        "agreeing -o/--format must produce a PNG"
    );
}

/// `--merged` on a multi-page document writes one `<base>.png` (exercises the
/// stacked-pixmap `to_png_merged` path), not per-page files.
#[test]
fn e2e_render_merged_png() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("doc.ncmp.typ");
    std::fs::write(&input, TWO_PAGE_TYP).expect("write input");
    let out = tmp.path().join("merged.png");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out), "--merged"])
        .assert()
        .success();

    let png = std::fs::read(&out).expect("read merged PNG");
    assert!(png_width(&png) > 0, "merged PNG must have positive width");
    assert!(
        !tmp.path().join("merged-1.png").exists(),
        "merged output must not produce per-page files"
    );
}

/// A multi-page document without `--merged` splits PNGs into `<base>-1.png` /
/// `<base>-2.png` and never writes the bare `<base>.png`.
#[test]
fn e2e_render_multipage_png_naming() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("doc.ncmp.typ");
    std::fs::write(&input, TWO_PAGE_TYP).expect("write input");
    let out = tmp.path().join("doc.png");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out)])
        .assert()
        .success();

    let page1 = tmp.path().join("doc-1.png");
    let page2 = tmp.path().join("doc-2.png");
    assert!(page1.exists(), "page 1 PNG must exist at {page1:?}");
    assert!(page2.exists(), "page 2 PNG must exist at {page2:?}");
    assert!(
        !out.exists(),
        "the bare base path must not be written for a multi-page split"
    );
    assert!(
        png_width(&std::fs::read(&page1).expect("read page 1")) > 0,
        "page 1 must be a real PNG"
    );
}

/// Failure path: when compilation fails, SVG export bails non-zero and writes
/// no output file.
#[test]
fn e2e_render_svg_export_compile_failure() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("broken.ncmp.typ");
    // `#undefined_symbol` is an unknown variable: a hard compile error.
    std::fs::write(&input, "= Title\n#undefined_symbol\n").expect("write broken input");
    let out = tmp.path().join("broken.svg");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out)])
        .assert()
        .failure()
        .stderr(predicate::str::contains("broken.ncmp.typ"));

    assert!(
        !out.exists(),
        "no SVG must be written when compilation fails"
    );
}

/// Failure path: an unrecognised `--format` value is rejected by clap.
#[test]
fn e2e_render_invalid_format_value() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "--format", "jpeg"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

/// Failure path: a non-numeric `--dpi` is rejected by clap.
#[test]
fn e2e_render_invalid_dpi_value() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let input = tmp.path().join("page.ncmp.typ");
    std::fs::write(&input, ONE_PAGE_TYP).expect("write input");
    let out = tmp.path().join("page.png");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["render", &abs(&input), "-o", &abs(&out), "--dpi", "lots"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

/// Build a two-page entry-format `.ndoc.typ` archive at `doc` via `new` + `add`,
/// returning nothing (the file is left on disk for the caller to build).
fn write_two_page_archive(doc: &Path) {
    let dir = doc.parent().expect("doc has a parent dir");
    let content = dir.join("two_page.typ");
    std::fs::write(&content, "Page one\n#pagebreak()\nPage two\n").expect("write entry content");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["new", &abs(doc)])
        .assert()
        .success();
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["add", &abs(doc), "main", "--content-file", &abs(&content)])
        .assert()
        .success();
}

/// `ndoc build <two-page.ndoc.typ> --format svg` splits into `<base>-1.svg` /
/// `<base>-2.svg`, mirroring `render`'s multi-page convention.
#[test]
fn e2e_build_multipage_svg_naming() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("paper.ndoc.typ");
    write_two_page_archive(&doc);

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&doc), "--format", "svg"])
        .assert()
        .success();

    let page1 = tmp.path().join("paper-1.svg");
    let page2 = tmp.path().join("paper-2.svg");
    assert!(page1.exists(), "page 1 SVG must exist at {page1:?}");
    assert!(page2.exists(), "page 2 SVG must exist at {page2:?}");
    assert!(
        !tmp.path().join("paper.svg").exists(),
        "the bare base path must not be written for a multi-page split"
    );
    let svg1 = std::fs::read_to_string(&page1).expect("read page 1");
    assert!(svg1.starts_with("<svg"), "page 1 must be an SVG");
}

/// `ndoc build <two-page.ndoc.typ> --format svg --merged` writes one merged SVG.
#[test]
fn e2e_build_merged_svg() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let doc = tmp.path().join("paper.ndoc.typ");
    write_two_page_archive(&doc);
    let merged = tmp.path().join("paper.svg");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&doc), "--format", "svg", "--merged"])
        .assert()
        .success();

    let svg = std::fs::read_to_string(&merged).expect("read merged SVG");
    assert!(
        svg.starts_with("<svg"),
        "merged build output must be an SVG"
    );
    assert!(
        !tmp.path().join("paper-1.svg").exists(),
        "merged build must not produce per-page files"
    );
}

/// `--dpi` scales `build`'s rasterised PNG just as it does for `render`.
#[test]
fn e2e_build_dpi_scales_png() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let low_md = tmp.path().join("low.md");
    let high_md = tmp.path().join("high.md");
    std::fs::write(&low_md, "# Heading\n\nBody.\n").expect("write low markdown");
    std::fs::write(&high_md, "# Heading\n\nBody.\n").expect("write high markdown");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&low_md), "--format", "png", "--dpi", "72"])
        .assert()
        .success();
    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&high_md), "--format", "png", "--dpi", "300"])
        .assert()
        .success();

    let low_w = png_width(&std::fs::read(tmp.path().join("low.png")).expect("read low png"));
    let high_w = png_width(&std::fs::read(tmp.path().join("high.png")).expect("read high png"));
    assert!(
        high_w > low_w,
        "higher build --dpi must widen the PNG ({high_w} !> {low_w})"
    );
}

/// Failure path: an unrecognised `--format` value on `build` is rejected.
#[test]
fn e2e_build_invalid_format_value() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let md = tmp.path().join("doc.md");
    std::fs::write(&md, "# Heading\n\nBody.\n").expect("write markdown");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["build", &abs(&md), "--format", "gif"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

/// `ndoc jump` without `--json` prints the resolved location as human text.
#[test]
fn e2e_jump_text_output() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/jump.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["jump", &abs(&input), "--page", "1", "--at", "13,14"])
        .assert()
        .success()
        .stdout(predicate::str::contains("file main.typ:2:2"));
}

/// Failure path: `--page 0` is rejected because page numbering is 1-based.
#[test]
fn e2e_jump_page_zero_fails() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = manifest.join("tests/fixtures/jump.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["jump", &abs(&input), "--page", "0", "--at", "13,14"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("1-based"));
}

/// Failure path: `ndoc jump` on a missing input names the offending path.
#[test]
fn e2e_jump_missing_input_fails() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let ghost = tmp.path().join("ghost.typ");

    Command::cargo_bin("ndoc")
        .expect("ndoc binary builds")
        .args(["jump", &abs(&ghost), "--page", "1", "--at", "13,14"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ghost.typ"));
}
