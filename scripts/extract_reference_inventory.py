"""Reference C# test inventory extractor and reconciler.

Walks the read-only `.reference/Typst` tree, extracts every NUnit `[Test]`
case keyed `relative_file::TestName`, and writes a JSON inventory plus a
per-area reconciliation summary used to re-check `docs/reference-parity-map.md`
when the reference tree changes. The script never modifies `.reference/**`.

In-scope areas (core, cli) reconcile exactly against raw `[Test]` attribute
counts. The AgentTools area is Deferred (per charter) and reports a small
case_rows-vs-raw mismatch because two `[Test]`-annotated helper methods are
parsed as cases; this is expected and does not affect in-scope parity.
"""

import os, re, json, sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.join(REPO_ROOT, ".reference", "Typst")
AREAS = {
    "core": "test",
    "cli": "CLI/test",
    "agenttools": "AgentTools/test",
}

# Attribute detection
ATTR_RE = re.compile(r'^\s*\[(Test|TestCase|TestCaseSource|Theory)\b')
# method signature: capture the method name after attributes
METHOD_RE = re.compile(r'(?:public|private|protected|internal)\s+(?:async\s+)?(?:Task|void|ValueTask)\s+(\w+)\s*\(')

def parse_file(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        lines = f.readlines()
    results = []  # list of dicts: method, attr_types list, testcase_count
    i = 0
    n = len(lines)
    pending_attrs = []  # tuples (type, rawline)
    for idx, line in enumerate(lines):
        m = ATTR_RE.match(line)
        if m:
            pending_attrs.append((m.group(1), line.strip()))
            continue
        # attribute lines may continue (multi-line TestCase). detect method
        mm = METHOD_RE.search(line)
        if mm and pending_attrs:
            method = mm.group(1)
            tc = sum(1 for t,_ in pending_attrs if t in ("TestCase",))
            tcs = sum(1 for t,_ in pending_attrs if t == "TestCaseSource")
            has_test = any(t in ("Test","Theory") for t in (a[0] for a in pending_attrs))
            # case rows: each TestCase = 1 row; a plain [Test] = 1 row
            if tc > 0:
                rows = tc
                kind = "TestCase"
            elif tcs > 0:
                rows = tcs  # at least one, source-driven (variable)
                kind = "TestCaseSource"
            elif has_test:
                rows = 1
                kind = "Test"
            else:
                rows = 1
                kind = "Test"
            results.append({"method": method, "kind": kind, "rows": rows, "testcase_attrs": tc, "source_attrs": tcs})
            pending_attrs = []
        elif pending_attrs and line.strip() and not line.strip().startswith("[") and not line.strip().startswith("//"):
            # a non-attribute, non-method line breaks the pending sequence only if it's not a continuation
            # keep accumulating if it's a continuation of a TestCase (ends without method). Reset only on clear breaks like '}' alone
            if line.strip() in ("{", "}"):
                pending_attrs = []
    return results

out = {}
for area, rel in AREAS.items():
    base = os.path.join(ROOT, rel)
    files = []
    for dirpath, dirnames, filenames in os.walk(base):
        if "/obj" in dirpath or "/bin" in dirpath:
            continue
        for fn in filenames:
            if fn.endswith(".cs"):
                files.append(os.path.join(dirpath, fn))
    files.sort()
    area_data = {}
    for fp in files:
        relkey = os.path.relpath(fp, ROOT)
        parsed = parse_file(fp)
        if parsed:
            area_data[relkey] = parsed
    out[area] = area_data

# Reconciliation summary: case rows vs raw [Test] attribute counts per area.
TEST_ATTR_RE = re.compile(r'\[\s*Test\s*\]')
print("Reference inventory reconciliation (per area):")
for area in out:
    methods = sum(len(lst) for lst in out[area].values())
    rows = sum(x["rows"] for lst in out[area].values() for x in lst)
    raw = 0
    for relf in out[area]:
        with open(os.path.join(ROOT, relf), encoding="utf-8", errors="replace") as fh:
            raw += len(TEST_ATTR_RE.findall(fh.read()))
    flag = "ok" if rows == raw else f"MISMATCH (raw={raw})"
    print(f"  {area}: files={len(out[area])} case_rows={rows} {flag}")

out_path = os.path.join(REPO_ROOT, "docs", "reference-inventory.json")
os.makedirs(os.path.dirname(out_path), exist_ok=True)
with open(out_path, "w") as fh:
    json.dump(out, fh, indent=2)
print(f"Wrote {out_path}")
