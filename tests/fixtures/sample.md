# Sample Document

A fixture for the `ndoc build` end-to-end tests.

## Overview

northdoc compiles Markdown to PDF via an embedded Typst engine.[^1]

## Feature Matrix

| Feature    | Status   | Notes         |
|:-----------|:--------:|:--------------|
| Headings   | Complete | H1 to H4      |
| Tables     | Complete | GFM alignment |
| Task lists | Complete | Checked items |
| Footnotes  | Complete | Two-pass      |

## Progress

- [x] Wire ndoc build command
- [x] Embed Typst compiler
- [ ] Add output flag

[^1]: No external Typst binary required.
