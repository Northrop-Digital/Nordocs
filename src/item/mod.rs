//! Reusable item collections and their validation.
//!
//! Items are data-driven content records authored as Markdown files with a YAML
//! frontmatter block. They seed component inputs from a reusable store: each item
//! declares the component schema it conforms to (`$schema`), the collection it
//! belongs to (`$collection`), optional tags, and a set of user inputs.
//!
//! Ports the core of the C# `ItemParser`/`ItemValidator`: discovery,
//! frontmatter parsing with reserved `$`-prefixed keys, and validation of an
//! item's inputs against its sibling component schema (required inputs present,
//! image inputs resolvable on disk). The heavier list-input/indexed-content
//! materialisation from the C# parser is intentionally omitted — v1 validates
//! the scalar input surface the redesign exercises.
//!
//! Validation looks up an item's `$schema` among the components in the same
//! directory (the `*.ncmp.typ` files loaded via [`schema::parse`](crate::schema::parse)),
//! so an items directory is self-describing: its schemas sit next to its items.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::model::InputKind;
use crate::schema::ComponentSchema;

/// File suffix identifying an item file.
const ITEM_SUFFIX: &str = ".item.md";

/// Opening/closing delimiter of an item's YAML frontmatter block.
const FRONTMATTER_FENCE: &str = "---";

/// A single parsed item record.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The component schema this item conforms to (`$schema`).
    pub schema: String,
    /// The collection this item belongs to (`$collection`).
    pub collection: String,
    /// Tags from `$tags`; empty when absent.
    pub tags: Vec<String>,
    /// User-defined inputs, in sorted key order. Values are the raw frontmatter
    /// scalars rendered to their string form for schema-driven validation.
    pub inputs: BTreeMap<String, String>,
    /// Absolute-or-relative path of the source file the item was parsed from.
    pub source_path: PathBuf,
}

/// A single validation problem found on an item.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemIssue {
    /// Path of the item file that produced the issue.
    pub source_path: PathBuf,
    /// Stable issue code (e.g. `unknown-schema`, `missing-input`, `missing-image`).
    pub code: &'static str,
    /// Human-readable message naming the offending field.
    pub message: String,
}

/// The frontmatter shape of an item file.
///
/// Reserved keys are `$`-prefixed; every other key is captured as a user input
/// via `#[serde(flatten)]`. `$schema`/`$collection` are required; missing either
/// is a typed parse error.
#[derive(Debug, Deserialize)]
struct ItemFrontmatter {
    #[serde(rename = "$schema")]
    schema: String,
    #[serde(rename = "$collection")]
    collection: String,
    #[serde(rename = "$tags", default)]
    tags: Vec<String>,
    #[serde(flatten)]
    inputs: BTreeMap<String, serde_yaml_ng::Value>,
}

/// Parse an item from raw Markdown source text.
///
/// Extracts the leading `---`-fenced YAML frontmatter and deserialises the
/// reserved keys plus user inputs. The Markdown body is not retained: v1
/// validation is input-driven, so the body carries no validated structure.
pub fn parse_item_str(content: &str, source_path: &Path) -> Result<Item> {
    let yaml = extract_frontmatter(content).ok_or_else(|| {
        Error::Schema(format!(
            "item '{}' is missing its leading '---' YAML frontmatter",
            source_path.display()
        ))
    })?;
    let front: ItemFrontmatter = serde_yaml_ng::from_str(yaml).map_err(|e| {
        Error::Schema(format!(
            "item '{}' has invalid frontmatter: {e}",
            source_path.display()
        ))
    })?;
    let inputs = front
        .inputs
        .into_iter()
        .map(|(k, v)| (k, scalar_to_string(&v)))
        .collect();
    Ok(Item {
        schema: front.schema,
        collection: front.collection,
        tags: front.tags,
        inputs,
        source_path: source_path.to_path_buf(),
    })
}

/// Parse an item from a file on disk.
pub fn parse_item_file(path: &Path) -> Result<Item> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Schema(format!("cannot read item '{}': {e}", path.display())))?;
    parse_item_str(&content, path)
}

/// Discover and parse every item (`*.item.md`) in a directory, in stable path
/// order.
///
/// Errors if the directory cannot be read or any item fails to parse. An empty
/// or item-free directory yields an empty list (not an error) so `load` can
/// report zero collections.
pub fn load_items_from_dir(dir: &Path) -> Result<Vec<Item>> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        Error::Schema(format!(
            "cannot read items directory '{}': {e}",
            dir.display()
        ))
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| Error::Schema(format!("cannot read directory entry: {e}")))?;
        let path = entry.path();
        let is_item = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(ITEM_SUFFIX));
        if is_item {
            paths.push(path);
        }
    }
    paths.sort();
    paths.iter().map(|p| parse_item_file(p)).collect()
}

/// Count items per collection, in sorted collection-name order.
///
/// Drives the `load` summary: each entry is `(collection, item_count)`.
pub fn summarise_collections(items: &[Item]) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for item in items {
        *counts.entry(item.collection.clone()).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

/// Validate every item against its sibling component schema.
///
/// For each item, looks up `$schema` among `components`; an unknown schema, a
/// missing required input, or an image-typed input whose referenced file does
/// not resolve next to the item all surface as an [`ItemIssue`] naming the
/// source path. Issues are returned in item-then-schema declaration order.
pub fn validate_items(items: &[Item], components: &[ComponentSchema]) -> Vec<ItemIssue> {
    let mut issues = Vec::new();
    for item in items {
        let Some(schema) = components.iter().find(|c| c.name == item.schema) else {
            issues.push(ItemIssue {
                source_path: item.source_path.clone(),
                code: "unknown-schema",
                message: format!(
                    "item declares '$schema: {}' which is not a component in the items directory",
                    item.schema
                ),
            });
            continue;
        };
        validate_item_against_schema(item, schema, &mut issues);
    }
    issues
}

/// Validate a single item's inputs against its resolved component schema.
fn validate_item_against_schema(
    item: &Item,
    schema: &ComponentSchema,
    issues: &mut Vec<ItemIssue>,
) {
    let item_dir = item.source_path.parent().unwrap_or_else(|| Path::new("."));
    for input in &schema.inputs {
        match item.inputs.get(&input.name) {
            None if input.required => issues.push(ItemIssue {
                source_path: item.source_path.clone(),
                code: "missing-input",
                message: format!("required input '{}' is missing", input.name),
            }),
            None => {}
            Some(value) if input.kind == InputKind::Image && !value.is_empty() => {
                let resolved = item_dir.join(value);
                if !resolved.exists() {
                    issues.push(ItemIssue {
                        source_path: item.source_path.clone(),
                        code: "missing-image",
                        message: format!(
                            "image input '{}' references '{value}' which does not exist at '{}'",
                            input.name,
                            resolved.display()
                        ),
                    });
                }
            }
            Some(_) => {}
        }
    }
}

/// Extract the YAML text of the leading `---`-fenced frontmatter block.
///
/// Returns `None` unless the file begins (after optional BOM/leading
/// whitespace) with a `---` line and a matching closing `---` line. Mirrors the
/// `.md` frontmatter fence used elsewhere in the toolset.
fn extract_frontmatter(content: &str) -> Option<&str> {
    let trimmed = content.trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n']);
    let after_open = trimmed.strip_prefix(FRONTMATTER_FENCE)?;
    let after_open = after_open
        .strip_prefix('\n')
        .or_else(|| after_open.strip_prefix("\r\n"))?;
    let close = find_closing_fence(after_open)?;
    Some(after_open[..close].trim_end_matches(['\n', '\r']))
}

/// Find the byte offset of a line containing only `---` (the closing fence).
fn find_closing_fence(rest: &str) -> Option<usize> {
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']) == FRONTMATTER_FENCE {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

/// Render a frontmatter scalar to the string form used for validation.
///
/// Scalars (string/number/bool) become their natural text; structured values
/// (sequences, mappings) fall back to a compact debug form, which is enough for
/// the scalar-input validation v1 performs.
fn scalar_to_string(value: &serde_yaml_ng::Value) -> String {
    match value {
        serde_yaml_ng::Value::String(s) => s.clone(),
        serde_yaml_ng::Value::Bool(b) => b.to_string(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        serde_yaml_ng::Value::Null => String::new(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::InputSchema;
    use std::io::Write;

    fn schema(name: &str, inputs: Vec<InputSchema>) -> ComponentSchema {
        ComponentSchema {
            name: name.to_string(),
            inputs,
            has_body: true,
            allowed_children: Vec::new(),
        }
    }

    fn required(name: &str, kind: InputKind) -> InputSchema {
        InputSchema {
            name: name.to_string(),
            kind,
            required: true,
        }
    }

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).expect("create fixture file");
        f.write_all(content.as_bytes()).expect("write fixture file");
        path
    }

    const VALID_ITEM: &str = "---\n\
        $schema: project\n\
        $collection: projects\n\
        $tags:\n\
        \x20 - flagship\n\
        title: Northwind\n\
        budget: 4200\n\
        ---\n\
        # body markdown ignored\n";

    #[test]
    fn parse_reads_reserved_keys_and_inputs() {
        let item = parse_item_str(VALID_ITEM, Path::new("a.item.md")).expect("parse item");
        assert_eq!(item.schema, "project");
        assert_eq!(item.collection, "projects");
        assert_eq!(item.tags, vec!["flagship".to_string()]);
        assert_eq!(
            item.inputs.get("title").map(String::as_str),
            Some("Northwind")
        );
        assert_eq!(item.inputs.get("budget").map(String::as_str), Some("4200"));
    }

    #[test]
    fn missing_frontmatter_is_typed_error() {
        let err = parse_item_str("# just markdown\n", Path::new("x.item.md"))
            .expect_err("must reject without frontmatter");
        assert!(matches!(err, Error::Schema(_)));
    }

    #[test]
    fn missing_required_reserved_key_is_typed_error() {
        let bad = "---\n$collection: projects\n---\nbody\n";
        let err =
            parse_item_str(bad, Path::new("x.item.md")).expect_err("missing $schema must error");
        assert!(matches!(err, Error::Schema(_)));
    }

    #[test]
    fn load_items_from_dir_is_stable_order_and_skips_non_items() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_file(dir.path(), "b.item.md", VALID_ITEM);
        write_file(dir.path(), "a.item.md", VALID_ITEM);
        write_file(dir.path(), "notes.md", "# not an item");

        let items = load_items_from_dir(dir.path()).expect("load items");
        let names: Vec<String> = items
            .iter()
            .map(|i| {
                i.source_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, vec!["a.item.md", "b.item.md"]);
    }

    #[test]
    fn load_items_missing_dir_is_typed_error() {
        let err = load_items_from_dir(Path::new("/no/such/items/dir"))
            .expect_err("missing dir must error");
        assert!(matches!(err, Error::Schema(_)));
    }

    #[test]
    fn summarise_groups_by_collection() {
        let items = vec![
            parse_item_str(VALID_ITEM, Path::new("a.item.md")).unwrap(),
            parse_item_str(
                &VALID_ITEM.replace("projects", "people"),
                Path::new("b.item.md"),
            )
            .unwrap(),
            parse_item_str(VALID_ITEM, Path::new("c.item.md")).unwrap(),
        ];
        let summary = summarise_collections(&items);
        assert_eq!(
            summary,
            vec![("people".to_string(), 1), ("projects".to_string(), 2)]
        );
    }

    #[test]
    fn validate_reports_unknown_schema() {
        let item = parse_item_str(VALID_ITEM, Path::new("a.item.md")).unwrap();
        let issues = validate_items(&[item], &[]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "unknown-schema");
        assert_eq!(issues[0].source_path, PathBuf::from("a.item.md"));
    }

    #[test]
    fn validate_reports_missing_required_input() {
        let item = parse_item_str(VALID_ITEM, Path::new("a.item.md")).unwrap();
        let comp = schema(
            "project",
            vec![
                required("title", InputKind::String),
                required("lead", InputKind::String),
            ],
        );
        let issues = validate_items(&[item], &[comp]);
        assert_eq!(issues.len(), 1, "title present, lead missing");
        assert_eq!(issues[0].code, "missing-input");
        assert!(issues[0].message.contains("lead"));
    }

    #[test]
    fn validate_passes_when_required_inputs_present() {
        let item = parse_item_str(VALID_ITEM, Path::new("a.item.md")).unwrap();
        let comp = schema("project", vec![required("title", InputKind::String)]);
        assert!(validate_items(&[item], &[comp]).is_empty());
    }

    #[test]
    fn validate_reports_missing_image() {
        let dir = tempfile::tempdir().expect("tempdir");
        let item_src = "---\n$schema: project\n$collection: projects\nlogo: missing.png\n---\n";
        let item_path = write_file(dir.path(), "a.item.md", item_src);
        let item = parse_item_file(&item_path).unwrap();
        let comp = schema("project", vec![required("logo", InputKind::Image)]);
        let issues = validate_items(&[item], &[comp]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "missing-image");
    }

    #[test]
    fn validate_resolves_present_image() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_file(dir.path(), "logo.png", "fake png bytes");
        let item_src = "---\n$schema: project\n$collection: projects\nlogo: logo.png\n---\n";
        let item_path = write_file(dir.path(), "a.item.md", item_src);
        let item = parse_item_file(&item_path).unwrap();
        let comp = schema("project", vec![required("logo", InputKind::Image)]);
        assert!(validate_items(&[item], &[comp]).is_empty());
    }
}
