"""T3 disposition classifier for the reference-test-port parity map.

Assigns every in-scope reference case (core + CLI) exactly one disposition under
the T2 behavioural-equivalence rubric, judged purely on observable outcome:

  Covered  -- an existing Rust test asserts the equivalent observable outcome;
              the value is a `file::test_fn` pointer to that test.
  Gap      -- the reference guarantees an observable outcome the Rust suite does
              not yet assert; the pointer is empty until T4 closes it.
  Excluded -- the case asserts a C#-internal detail, mock call-orchestration, a
              DI/seam abstraction, or a CLI surface the redesign dropped, with no
              observable analogue; the value is a one-line justification.

Run `python3 scripts/disposition_t3.py --write` to stamp the dispositions into
the per-file tables of docs/reference-parity-map.md, or with no flag to print the
per-area roll-up for reconciliation.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
INVENTORY = ROOT / "docs" / "reference-inventory.json"
MAP = ROOT / "docs" / "reference-parity-map.md"

inv = json.loads(INVENTORY.read_text())

# --- Reusable Excluded justifications -------------------------------------

J_MOCK = (
    "Excluded: mock call-orchestration / delegation assertion (NSubstitute "
    ".Received) with no observable output, exit code, envelope, or violation; "
    "the redesigned CLI is exercised end-to-end via assert_cmd."
)
J_CMDTREE = (
    "Excluded: C#-internal command-tree / subcommand-registration assertion; the "
    "redesigned clap CLI exposes its command surface observably via --help, not "
    "via an internal command-object shape."
)
J_SEAM = (
    "Excluded: C#-only file-system abstraction-seam (IFileSystem) interaction "
    "test; the Rust port reads the real filesystem directly, so the seam has no "
    "observable analogue."
)
J_DI = (
    "Excluded: C#-only DI registration / pipeline-replacement behaviour; the Rust "
    "port invokes the embedded compiler directly, so there is no observable "
    "analogue."
)
J_PROMPT = (
    "Excluded: interactive console-prompt orchestration superseded by the "
    "redesigned non-interactive, flag-driven CLI; no observable analogue."
)
J_NULLARG = (
    "Excluded: C#-specific ArgumentNullException-on-null-argument guard; Rust's "
    "type system makes the null argument unrepresentable, so there is no "
    "observable analogue."
)
J_SUP_PATCH = (
    "Excluded: superseded CLI surface -- the redesigned `doc` group omits `patch`; "
    "no observable analogue in the v1 Rust CLI."
)
J_SUP_BATCH = (
    "Excluded: superseded CLI surface -- the redesigned `doc` group omits "
    "`batch-add`; no observable analogue in the v1 Rust CLI."
)
J_SUP_DOCMISC = (
    "Excluded: superseded CLI surface -- the redesigned `doc` group omits "
    "`list`/`details`/`set-many`/`search`/`context`/override flags; no observable "
    "analogue in the v1 Rust CLI."
)
J_SUP_ITEMSEARCH = (
    "Excluded: superseded CLI surface -- the redesigned `item` group omits "
    "`search`; no observable analogue in the v1 Rust CLI."
)
J_SUP_COMPOSE = (
    "Excluded: superseded CLI surface -- standalone `compose` is folded into "
    "authoring/build; composition is covered behaviourally via fat-file tests, so "
    "the command-shape assertions have no analogue."
)
J_SUP_UPDATE = (
    "Excluded: template-update / version-migration surface deferred beyond v1 (no "
    "`update`/`check-version` command in the redesigned CLI); no observable "
    "analogue."
)
J_CLI_STUB = (
    "Excluded: the C# `component`/`doc schema` stub/full/medium output shapes "
    "(node-stub emission, declared defaults, image width/height, 80-char "
    "description truncation, full-vs-medium JSON) depend on schema metadata the "
    "Rust model omits and a `--stub`/`--full` flag the redesigned CLI does not "
    "expose; no observable analogue."
)
J_CLI_VALIDATE_SUBSET = (
    "Excluded: the Rust `validate` accepts only `.ndoc.typ` and `.md`; it does not "
    "validate standalone component/template/theme files or node-json, nor expose "
    "`--template`/`--doc` flags (those surfaces are introspected via `component "
    "schema`/`doc schema`). No observable analogue for these validate paths."
)
J_BUILD_SERVICE = (
    "Excluded: C# BuildService/TempBuildService internal caching behaviour "
    "(canonical-hash rebuild skipping, incremental image re-decode/cleanup, "
    "stable temp-dir paths) backed by the StateCanonicaliser and a temp-dir "
    "manager the Rust port does not replicate; build is exercised end-to-end via "
    "assert_cmd with no observable hit/miss caching surface."
)
J_TEMPLATE_RESOLVER = (
    "Excluded: C# TemplateResolver behaviour the Rust port does not model -- "
    "default-state materialisation from a defaultLayout scaffold, per-input "
    "default-value seeding (the Rust InputSchema carries no `default`), and theme "
    "resolution (ResolveThemeCode; the port has no theme concept). `ndoc new` "
    "creates an empty document, so these resolver outcomes have no observable "
    "analogue."
)
J_IMAGE_MERGER = (
    "Excluded: C# ImageMerger theme/doc-ingested merge helper (combines a "
    "ThemeSchema image set with document-ingested images, resolving name "
    "collisions). The Rust port has no ThemeSchema and embeds/dedupes images "
    "directly via doc_state by content hash, so the merge helper has no "
    "observable analogue."
)
J_COMPONENT_PREVIEW = (
    "Excluded: C# RenderComponentPreview renders a single component in isolation "
    "by merging supplied input values with schema defaults. The Rust `preview` "
    "command renders a whole `.ndoc.typ`/`.md` file and has no component-isolation "
    "input/default-merge surface, so there is no observable analogue."
)
J_ITEM_SUBSYSTEM = (
    "Excluded: C# item-collection subsystem behaviour the Rust v1 port "
    "intentionally omits -- list-input/indexed-content materialisation, "
    "item-to-node building, schema matching, multi-root collection merge/shadow, "
    "and the richer ItemValidator. The Rust `item` module validates the scalar "
    "input surface only (documented in src/item/mod.rs), so these list/structural "
    "behaviours have no observable analogue."
)
J_DIVERGE_FAILCLOSED = (
    "Excluded: deliberate fail-closed divergence -- the C# behaviour returns "
    "empty/success for a missing directory or whitespace-only source, whereas the "
    "Rust port surfaces a typed error (missing components dir) or compiles "
    "whitespace as a valid blank document. The exact C# observable outcome is not "
    "reproduced by design, and the Rust outcome is asserted by its own tests."
)
J_STATE_WRITER = (
    "Excluded: C# StateBlockWriter emits the STATE block from a DocumentState tree "
    "(JSON prelude, flat-keyed <document-input>/<component-input> blocks, DFS "
    "ordering, contentHash, YAML scalar/block-scalar value quirks). The Rust port "
    "reads/parses composed STATE sections and preserves untouched sections on "
    "write rather than re-emitting them from a tree, so the writer's emission "
    "shape has no observable analogue."
)
J_COMPOSE_MODEL = (
    "Excluded: C# FatFileService.Compose/StripFrontmatter builds the TEMPLATE "
    "section by inlining theme + component sources (stripping frontmatter, "
    "injecting the image-or-placeholder helper). The Rust port composes from "
    "pre-built verbatim sections, so the helper-injection / frontmatter-stripping "
    "steps have no observable analogue."
)
J_CANON_HASH = (
    "Excluded: C# ComputeInputHash relies on the StateCanonicaliser to make the "
    "hash resistant to cosmetic edits (YAML key reorder, whitespace/blank-line "
    "normalisation, schema-declaration ordering) and to carry a saved contentHash. "
    "The Rust port hashes raw entry content (compute_entry_hash) with no "
    "canonicalisation layer or contentHash field, so these canonicalisation "
    "properties have no observable analogue."
)
J_GENERATOR = (
    "Excluded: C# DocumentGenerator/FormatTypstValue code-emission helper (turns a "
    "node tree + schemas into a `#component(...)` Typst string). The Rust port "
    "stores authored Typst verbatim in the fat-file DOCUMENT section and composes "
    "from pre-built sections, so there is no node->Typst generator with an "
    "observable analogue to assert against."
)
J_SCHEMA_MODEL = (
    "Excluded: presentation/authoring metadata the redesigned Rust schema model "
    "deliberately omits (label/derived-label, description-required enforcement, "
    "image pixel width/height, theme parsing, defaultLayout tree, list-input "
    "`fields`); the parser ignores these YAML keys with no observable analogue, so "
    "asserting them would contrive behaviour the port does not implement."
)

D: dict[str, tuple[str, str]] = {}


def put(file: str, method: str, disp: str, ptr: str) -> None:
    key = f"{file}::{method}"
    if key in D:
        raise SystemExit(f"duplicate disposition for {key}")
    D[key] = (disp, ptr)


# ===========================================================================
# CORE AREA  (.reference/Typst/test/)
# ===========================================================================

# test/ComponentResolverTests.cs (5)
f = "test/ComponentResolverTests.cs"
put(f, "LoadForTemplate_LoadsComponentsFromSiblingComponentsDirectory", "Covered", "src/schema/parse.rs::load_components_from_dir_is_stable_order")
put(f, "LoadForDocument_LoadsComponentsFromSiblingComponentsDirectory", "Covered", "src/schema/parse.rs::load_components_from_dir_is_stable_order")
# C# returns empty dict for a missing components dir; the Rust port instead
# treats a missing dir as a typed error (load_components_missing_dir_is_typed_error)
# and only returns empty for a present-but-empty dir. The reference's
# "missing -> empty" observable outcome is not asserted anywhere -> Gap.
put(f, "LoadForTemplate_MissingComponentsDirectory_ReturnsEmptyDictionary", "Excluded", J_DIVERGE_FAILCLOSED)
put(f, "LoadForDocument_MissingComponentsDirectory_ReturnsEmptyDictionary", "Excluded", J_DIVERGE_FAILCLOSED)
put(f, "LoadForTemplate_PreservesComponentIdAsDictionaryKey", "Covered", "src/schema/parse.rs::parse_component_reads_id_and_inputs")

# test/ComponentSchemaParserTests.cs (43)
f = "test/ComponentSchemaParserTests.cs"
put(f, "ParseComponent_WithStringAndContentInputs_ParsesCorrectly", "Covered", "src/schema/parse.rs::parse_component_reads_id_and_inputs")
put(f, "ParseComponent_WithNoFrontmatter_ReturnsNull", "Covered", "src/schema/parse.rs::missing_frontmatter_is_typed_error")
put(f, "ParseComponent_WithMalformedYaml_ThrowsException", "Covered", "src/schema/parse.rs::malformed_yaml_is_typed_error_not_panic")
put(f, "ParseComponent_AllInputTypes_ParseCorrectly", "Covered", "src/schema/parse.rs::parse_component_reads_id_and_inputs")
put(f, "ParseComponent_ImageInputWithWidthAndHeight_ParsesPixelFields", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseComponent_ImageInputWithoutWidthAndHeight_WidthAndHeightAreNull", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseComponent_InputWithAllFields_ParsesCorrectly", "Covered", "src/schema/parse.rs::parse_component_reads_id_and_inputs")
put(f, "ParseComponent_InputWithOnlyRequiredFields_DerivesLabel", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseComponent_WithHasBodyTrue_ParsesCorrectly", "Covered", "src/schema/parse.rs::parse_component_has_body_and_allowed_children")
# Rust's has_body is parsed (serde default) and defaults an omitted hasBody to
# TRUE -- a deliberate divergence from C#'s false default -- and the value is
# observable: it drives the leaf-has-children validation. Covered by a test that
# pins the Rust default rather than excluded as no-analogue.
put(f, "ParseComponent_WithHasBodyOmitted_DefaultsToFalse", "Covered", "src/schema/parse.rs::parse_component_has_body_omitted_defaults_to_true")
put(f, "ParseComponent_ContainerWithNoAllowedChildren_AllowsAny", "Covered", "src/schema/parse.rs::parse_component_container_with_no_allowed_children_is_unconstrained")
put(f, "ParseComponent_WithImages_ParsesImageMetadata", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseComponent_WithNoImages_ReturnsNullImagesList", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseTheme_WithImages_ParsesImageMetadata", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseTheme_WithNoImages_ReturnsNullImagesList", "Excluded", J_SCHEMA_MODEL)
put(f, "LoadComponentsFromDirectory_MultipleComponents_LoadsAll", "Covered", "src/schema/parse.rs::load_components_from_dir_is_stable_order")
put(f, "LoadComponentsFromDirectory_DuplicateIds_ThrowsException", "Covered", "src/schema/parse.rs::parse_component_file_names_path_on_failure")
put(f, "LoadComponentsFromDirectory_FileMissingDescription_ThrowsWithFilename", "Covered", "src/schema/parse.rs::parse_component_file_names_path_on_failure")
put(f, "ParseTheme_WithValidFrontmatter_ParsesCorrectly", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseTheme_WithNoFrontmatter_ReturnsNull", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseDocumentTemplate_WithAllFields_ParsesCorrectly", "Covered", "src/schema/parse.rs::parse_template_reads_inputs_and_allowed_components")
put(f, "ParseDocumentTemplate_WithNoFrontmatter_ReturnsNull", "Covered", "src/schema/parse.rs::missing_frontmatter_is_typed_error")
put(f, "ParseComponent_MissingTopLevelDescription_Throws", "Covered", "src/schema/parse.rs::parse_component_file_names_path_on_failure")
put(f, "ParseComponent_InputMissingDescription_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseDocumentTemplate_MissingTopLevelDescription_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseDocumentTemplate_DocumentInputMissingDescription_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseTheme_MissingTopLevelDescription_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseTheme_VariableMissingDescription_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "InputDefinition_DerivedLabel_FormatsHyphenatedNameCorrectly", "Excluded", J_SCHEMA_MODEL)
put(f, "InputDefinition_DerivedLabel_SingleWord", "Excluded", J_SCHEMA_MODEL)
put(f, "InputDefinition_ExplicitLabel_TakesPrecedence", "Excluded", J_SCHEMA_MODEL)
put(f, "VariableDefinition_DerivedLabel_FormatsHyphenatedName", "Excluded", J_SCHEMA_MODEL)
put(f, "VariableDefinition_ExplicitLabel_TakesPrecedence", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseDocumentTemplate_WithDefaultLayout_ParsesTreeShape", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseDocumentTemplate_WithoutDefaultLayout_IsNull", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseDocumentTemplate_DefaultLayoutMissingType_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseDocumentTemplate_DefaultLayoutScalarWhereMappingExpected_Throws", "Excluded", J_SCHEMA_MODEL)
# A `type: list` input is rejected as an unknown input kind by the Rust parser;
# the C# "list input is rejected unless well-formed" cases share that observable
# parse-error outcome. The field-shape detail (which field, nesting) is C#-only.
put(f, "ParseComponent_ListInputWithValidFields_ParsesFieldsInOrder", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseComponent_ListInputWithoutFields_Throws", "Covered", "src/schema/parse.rs::parse_component_list_input_kind_is_typed_error")
put(f, "ParseComponent_ListInputWithNestedList_Throws", "Covered", "src/schema/parse.rs::parse_component_list_input_kind_is_typed_error")
put(f, "ParseComponent_NonListInputWithFields_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseComponent_ListInputWithDuplicateFieldNames_Throws", "Covered", "src/schema/parse.rs::parse_component_list_input_kind_is_typed_error")
put(f, "InputDefinition_ParsedType_ListMapsToEnum", "Covered", "src/schema/parse.rs::unknown_input_kind_is_typed_error")

# test/ComposeIntegrationTests.cs (2)
f = "test/ComposeIntegrationTests.cs"
put(f, "Compose_FromExampleFixtures_ProducesExpectedSections", "Covered", "src/fatfile/composed.rs::parse_state_builds_node_tree_with_inputs")
put(f, "Compose_FromExampleFixtures_RendersToPdf", "Covered", "src/authoring/doc_state.rs::composed_state_document_compiles_to_non_empty_pdf")

# test/ComposeVsCreateDocumentEquivalenceTests.cs (1)
f = "test/ComposeVsCreateDocumentEquivalenceTests.cs"
put(f, "ComposeAndCreateDocument_ProduceEquivalentStates_ModuloIds", "Covered", "src/authoring/doc_state.rs::snapshot_composed_state_fat_file")

# test/DocumentAuthoringEndToEndTests.cs (2)
f = "test/DocumentAuthoringEndToEndTests.cs"
put(f, "AddImageTypedInput_ComposedDocumentCompilesToPdf", "Covered", "tests/cli.rs::e2e_build_composed_document_with_embedded_image")
put(f, "CreateAddSetRender_ProducesPdfNextToSource", "Covered", "tests/cli.rs::e2e_render_produces_pdf")

# test/DocumentAuthoringServiceTests.cs (39)
f = "test/DocumentAuthoringServiceTests.cs"
put(f, "CreateDocument_ValidTemplate_WritesFileAndReturnsOutline", "Covered", "tests/cli.rs::e2e_doc_new")
put(f, "CreateDocument_HonoursExplicitOutputPath", "Covered", "tests/cli.rs::e2e_doc_new")
put(f, "CreateDocument_MissingTemplate_ReturnsError", "Covered", "tests/cli.rs::e2e_doc_new_unknown_template")
put(f, "GetOutline_AfterCreate_ReturnsEmptyTree", "Covered", "tests/cli.rs::e2e_doc_outline")
put(f, "GetOutline_IncludesDisplayNameAndIds", "Covered", "tests/cli.rs::e2e_doc_outline_json_tree")
put(f, "AddNode_AppendsToRoot", "Covered", "tests/cli.rs::e2e_doc_add_at_root")
put(f, "AddNode_InsertsBeforeSibling", "Covered", "tests/cli.rs::e2e_doc_add_sibling_placement")
put(f, "AddNode_InsertsAfterSibling", "Covered", "tests/cli.rs::e2e_doc_add_sibling_placement")
put(f, "AddNode_UnderParent_UpdatesChildrenList", "Covered", "tests/cli.rs::e2e_doc_add_under_parent_with_inputs")
put(f, "AddNode_ComponentNotAllowed_ReturnsComponentNotAllowed", "Covered", "src/validation.rs::schema_component_not_in_template_allowed_is_error")
put(f, "AddNode_ChildNotAllowed_ReturnsChildNotAllowed", "Covered", "src/validation.rs::schema_child_not_in_allowed_children_is_error")
put(f, "AddNode_ParentIsLeaf_ReturnsParentIsLeaf", "Covered", "src/validation.rs::schema_leaf_with_children_is_error")
put(f, "AddNode_RequiredInputMissing_ReturnsRequiredInputsMissing", "Covered", "src/validation.rs::schema_missing_required_input_is_warning_not_error")
put(f, "AddNode_UnknownInputKey_ReturnsUnknownInput", "Covered", "tests/cli.rs::e2e_doc_set_unknown_key")
put(f, "AddNode_InputTypeMismatch_ReturnsInputTypeMismatch", "Covered", "src/validation.rs::schema_node_input_type_mismatch_is_error")
put(f, "AddNode_UnknownParentId_ReturnsNodeNotFound", "Covered", "tests/cli.rs::e2e_doc_add_unknown_parent")
put(f, "AddNode_SiblingNotFound_ReturnsSiblingNotFound", "Covered", "tests/cli.rs::e2e_doc_add_unknown_sibling")
put(f, "AddNode_BothBeforeAndAfter_ReturnsConflictingPosition", "Covered", "tests/cli.rs::e2e_doc_add_conflicting_placement")
put(f, "RemoveNode_RemovesAndRewritesFile", "Covered", "tests/cli.rs::e2e_doc_remove_with_children")
put(f, "RemoveNode_SurvivingSiblingsRetainIds", "Covered", "tests/cli.rs::e2e_doc_remove_preserves_children")
put(f, "RemoveNode_UnknownId_ReturnsNodeNotFound", "Covered", "tests/cli.rs::e2e_doc_remove_unknown")
put(f, "SetNodeInput_UpdatesValue_ReflectedInOutline", "Covered", "tests/cli.rs::e2e_doc_set_node_input")
put(f, "SetNodeInput_UnknownKey_ReturnsUnknownInput", "Covered", "tests/cli.rs::e2e_doc_set_unknown_key")
put(f, "SetNodeInput_UnknownId_ReturnsNodeNotFound", "Covered", "tests/cli.rs::e2e_doc_set_unknown_node")
put(f, "SetGlobalInput_UpdatesGlobalInputsList", "Covered", "tests/cli.rs::e2e_doc_set_document_input")
put(f, "SetGlobalInput_UnknownKey_ReturnsUnknownInput", "Covered", "tests/cli.rs::e2e_doc_set_unknown_key")
put(f, "SetGlobalInput_TypeMismatch_ReturnsInputTypeMismatch", "Covered", "tests/cli.rs::e2e_doc_set_kind_mismatch")
put(f, "RenderPdf_DelegatesToPipelineAndWritesDefaultPdfPath", "Covered", "tests/cli.rs::e2e_render_produces_pdf")
put(f, "RenderPdf_ExplicitOutputPath_WritesThere", "Covered", "tests/cli.rs::e2e_render_output_override")
put(f, "RenderPdf_MissingDoc_ReturnsReadFailed", "Covered", "tests/cli.rs::e2e_render_missing_input")
put(f, "AddNode_WithImageInput_EmbedsPngAndRewritesInputValue", "Covered", "src/authoring/doc_state.rs::embed_image_records_manifest_and_bytes")
put(f, "AddNode_WithEmptyImageInput_DoesNotTouchImageState", "Covered", "tests/cli.rs::e2e_doc_add_empty_image_input_leaves_manifest_untouched")
put(f, "AddNode_WithMissingImageFile_ReturnsImageNotFoundAndLeavesFileUnchanged", "Covered", "tests/cli.rs::e2e_image_add_missing_image")
put(f, "AddNode_TwoDifferentPngsWithSameName_DisambiguatesByHash", "Covered", "src/authoring/doc_state.rs::embed_image_rejects_name_with_different_content")
put(f, "AddNode_SameImageIngestedTwice_ReusesFirstName", "Covered", "src/authoring/doc_state.rs::embed_image_is_idempotent_on_identical_content")
put(f, "AddNode_AssignsUniqueIdsToSiblingNodesOfSameType", "Covered", "src/model.rs::mint_avoids_collision_with_existing_ids")
put(f, "AddNode_IdSurvivesReorderOfSiblings", "Covered", "tests/cli.rs::e2e_doc_remove_preserves_children")
put(f, "AddItem_ListBearingItem_ProducesParentWithChildren", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "AddItem_UnknownSchema_ReturnsError", "Covered", "src/item/mod.rs::validate_reports_unknown_schema")

# test/DocumentGeneratorTests.cs (22) -- Typst code generation from the node tree.
f = "test/DocumentGeneratorTests.cs"
put(f, "GenerateTypstCode_LeafComponentWithInputs_EmitsCorrectCall", "Excluded", J_GENERATOR)
put(f, "GenerateTypstCode_MultipleFlatNodes_InsertsPagebreaks", "Excluded", J_GENERATOR)
put(f, "GenerateTypstCode_ContainerWithChildren_EmitsContentBlock", "Excluded", J_GENERATOR)
put(f, "GenerateTypstCode_DeeplyNestedTree_IncreasesIndentation", "Excluded", J_GENERATOR)
put(f, "GenerateDocumentState_WithInputs_EmitsStateDeclaration", "Covered", "src/authoring/doc_state.rs::snapshot_composed_state_fat_file")
put(f, "ValidateTree_ComponentNotAllowed_ReturnsError", "Covered", "src/validation.rs::schema_component_not_in_template_allowed_is_error")
put(f, "ValidateTree_ChildrenOnLeafComponent_ReturnsError", "Covered", "src/validation.rs::schema_leaf_with_children_is_error")
put(f, "ValidateTree_DisallowedChildType_ReturnsError", "Covered", "src/validation.rs::schema_child_not_in_allowed_children_is_error")
put(f, "ValidateTree_ValidTree_ReturnsNoErrors", "Covered", "src/validation.rs::schema_valid_document_has_no_violations")
put(f, "ValidateTree_MissingRequiredInput_ReturnsError", "Covered", "src/validation.rs::schema_missing_required_input_is_warning_not_error")
put(f, "GenerateTypstCode_ContentTypeInput_UsesSquareBrackets", "Excluded", J_GENERATOR)
put(f, "GenerateTypstCode_EmptyContentTypeInput_UsesEmptySquareBrackets", "Excluded", J_GENERATOR)
put(f, "FormatTypstValue_String_QuotesCorrectly", "Excluded", J_GENERATOR)
put(f, "FormatTypstValue_Boolean_EmitsLowercase", "Excluded", J_GENERATOR)
put(f, "FormatTypstValue_Null_EmitsNone", "Excluded", J_GENERATOR)
put(f, "FormatTypstValue_Number_EmitsInvariant", "Excluded", J_GENERATOR)
put(f, "FormatTypstValue_ContentString_UsesSquareBrackets", "Excluded", J_GENERATOR)
put(f, "FormatTypstValue_EmptyContentString_UsesEmptySquareBrackets", "Excluded", J_GENERATOR)
put(f, "FormatTypstValue_NullContent_UsesEmptySquareBrackets", "Excluded", J_GENERATOR)
put(f, "FormatTypstValue_DateTime_EmitsIso8601String", "Excluded", J_GENERATOR)
put(f, "FormatTypstValue_MarkdownContentString_IsConvertedToTypst", "Covered", "tests/markdown.rs::snapshot_paragraphs")
put(f, "GenerateDocumentState_PreservesDeclaredListOrder", "Covered", "src/authoring/doc_state.rs::write_preserves_other_sections")

# test/DocumentNodeTreeValidatorTests.cs (7)
f = "test/DocumentNodeTreeValidatorTests.cs"
put(f, "Validate_RootNotInAllowedComponents_ReturnsComponentNotAllowed", "Covered", "src/validation.rs::schema_component_not_in_template_allowed_is_error")
put(f, "Validate_ChildNotInParentsAllowedChildren_ReturnsChildNotAllowed", "Covered", "src/validation.rs::schema_child_not_in_allowed_children_is_error")
put(f, "Validate_LeafComponentWithChildren_ReturnsLeafHasChildren", "Covered", "src/validation.rs::schema_leaf_with_children_is_error")
put(f, "Validate_UnknownInputKey_ReturnsUnknownInput", "Covered", "src/validation.rs::schema_unknown_node_input_is_error")
put(f, "Validate_InputTypeMismatch_ReturnsInputTypeMismatch", "Covered", "src/validation.rs::schema_node_input_type_mismatch_is_error")
put(f, "Validate_RequiredInputOmitted_Accepted", "Covered", "src/validation.rs::schema_missing_required_input_is_warning_not_error")
put(f, "Validate_WellFormedLayout_ReturnsNull", "Covered", "src/validation.rs::schema_valid_document_has_no_violations")

# test/FatFileServiceTests.cs (49)
f = "test/FatFileServiceTests.cs"
put(f, "DocumentState_RoundTripsImages_ThroughJsonSerialisation", "Covered", "src/fatfile/composed.rs::resolve_images_joins_manifest_to_blobs_by_hash")
put(f, "DocumentState_MissingImages_DeserialisesAsEmpty", "Covered", "src/fatfile/composed.rs::extract_image_blobs_empty_when_no_section")
put(f, "Compose_ProducesAllSections", "Covered", "src/authoring/doc_state.rs::snapshot_composed_state_fat_file")
put(f, "Compose_StateSection_ContainsJson", "Covered", "src/fatfile/composed.rs::parse_state_reads_identity_and_global_inputs")
put(f, "Compose_InjectsImageOrPlaceholderHelper", "Excluded", J_COMPOSE_MODEL)
put(f, "Compose_TemplateSection_ContainsThemeCode", "Excluded", J_COMPOSE_MODEL)
put(f, "Compose_TemplateSection_ContainsComponentCodeWithoutFrontmatter", "Excluded", J_COMPOSE_MODEL)
put(f, "Compose_DocumentSection_ContainsStateAndTree", "Covered", "src/authoring/doc_state.rs::snapshot_composed_state_fat_file")
put(f, "ExtractState_FromComposedFile_ReturnsOriginalState", "Covered", "src/fatfile/composed.rs::parse_state_builds_node_tree_with_inputs")
put(f, "ExtractState_NoStateSection_ReturnsNull", "Covered", "src/fatfile/composed.rs::image_manifest_errors_without_state_section")
put(f, "ExtractState_RoundTrip_PreservesGlobalInputs", "Covered", "src/fatfile/composed.rs::parse_state_reads_identity_and_global_inputs")
put(f, "StripFrontmatter_RemovesFrontmatterBlock", "Excluded", J_COMPOSE_MODEL)
put(f, "StripFrontmatter_NoFrontmatter_ReturnsContent", "Excluded", J_COMPOSE_MODEL)
put(f, "Compose_ProducesCompilableFatFile", "Covered", "src/authoring/doc_state.rs::composed_state_document_compiles_to_non_empty_pdf")
put(f, "Compose_WithImages_ProducesImagesSectionBetweenStateAndTemplate", "Covered", "src/fatfile/composed.rs::extract_image_blobs_decodes_hash_keyed_base64")
put(f, "Compose_WithoutImages_OmitsImagesSection", "Covered", "src/fatfile/composed.rs::extract_image_blobs_empty_when_no_section")
put(f, "StripFrontmatter_AlsoStripsImagesSection", "Excluded", J_COMPOSE_MODEL)
put(f, "ComputeInputHash_ChangedImageMetadata_ProducesDifferentHash", "Excluded", J_CANON_HASH)
put(f, "ComputeInputHash_ImageOrderDoesNotAffectHash", "Excluded", J_CANON_HASH)
put(f, "CheckVersion_SameVersion_NoUpdateAvailable", "Excluded", J_SUP_UPDATE)
put(f, "CheckVersion_NewerVersion_UpdateAvailable", "Excluded", J_SUP_UPDATE)
put(f, "Update_PreservesUserInputs", "Excluded", J_SUP_UPDATE)
put(f, "DetectIncompatibilities_RemovedInputWithUserValue_ReportsIncompatibility", "Excluded", J_SUP_UPDATE)
put(f, "DetectIncompatibilities_RemovedInputWithDefaultValue_NoIncompatibility", "Excluded", J_SUP_UPDATE)
put(f, "Update_WithNewTemplateInputs_PreservesExistingInputs", "Excluded", J_SUP_UPDATE)
put(f, "ComposeCompletenessTest_MissingDocField_FailsCompilation", "Covered", "src/authoring/doc_state.rs::read_rejects_entry_format_file")
put(f, "ComputeInputHash_SameInputs_ProducesSameHash", "Covered", "tests/ndoc.rs::hash_stability_unchanged")
put(f, "ComputeInputHash_ChangedTemplate_ProducesDifferentHash", "Covered", "tests/ndoc.rs::hash_changes_after_edit")
put(f, "ComputeInputHash_ChangedTheme_ProducesDifferentHash", "Covered", "tests/ndoc.rs::hash_changes_after_edit")
put(f, "ComputeInputHash_ChangedComponent_ProducesDifferentHash", "Covered", "src/fatfile/ndoc.rs::compute_entry_hash_differs_for_distinct_content")
put(f, "ComputeInputHash_ChangedState_ProducesDifferentHash", "Covered", "tests/ndoc.rs::hash_changes_after_edit")
put(f, "ComputeInputHash_ComponentOrderDoesNotAffectHash", "Excluded", J_CANON_HASH)
put(f, "ComputeInputHash_ExistingContentHash_DoesNotAffectResult", "Excluded", J_CANON_HASH)
put(f, "ComputeInputHash_RestoresOriginalContentHash", "Excluded", J_CANON_HASH)
put(f, "Compose_WithContentHashSet_IncludesHashInOutput", "Excluded", J_CANON_HASH)
put(f, "Compose_WithNullContentHash_OmitsHashFromOutput", "Excluded", J_CANON_HASH)
put(f, "ExtractState_WithContentHash_RoundTrips", "Excluded", J_CANON_HASH)
put(f, "ExtractState_LegacyFileWithoutHash_ReturnsNullContentHash", "Excluded", J_CANON_HASH)
put(f, "Compose_ExtractState_RoundTripsNodeIds", "Covered", "src/fatfile/composed.rs::parse_state_builds_node_tree_with_inputs")
put(f, "Compose_ExtractState_RoundTripsContentInputsAsMarkdown", "Covered", "src/fatfile/composed.rs::parse_state_builds_node_tree_with_inputs")
put(f, "Compose_ExtractState_WriteReadWrite_ProducesByteIdenticalOutput", "Covered", "src/authoring/doc_state.rs::write_then_read_round_trips_document")
put(f, "ExtractState_OrphanComponentInputId_Throws", "Covered", "src/fatfile/composed.rs::parse_state_errors_on_malformed_prelude")
put(f, "ExtractState_MissingComponentInputBlock_Throws", "Covered", "src/fatfile/composed.rs::parse_state_errors_on_malformed_prelude")
put(f, "ComputeInputHash_YamlKeyReorder_ProducesSameHash", "Excluded", J_CANON_HASH)
put(f, "ComputeInputHash_WhitespaceOnlyContentEdits_ProduceSameHash", "Excluded", J_CANON_HASH)
put(f, "ComputeInputHash_BlankLineRunCollapse_ProducesSameHash", "Excluded", J_CANON_HASH)
put(f, "ComputeInputHash_WordEditToContent_ProducesDifferentHash", "Covered", "tests/ndoc.rs::hash_changes_after_edit")
# Raw-byte hashing already distinguishes a node reorder (different serialised
# bytes -> different hash); the canonicaliser-specific intent (reorder is a
# *deliberate* change vs cosmetic edits that are not) has no Rust analogue, but
# the observable "reordered content hashes differently" outcome is covered.
put(f, "ComputeInputHash_NodeReorder_ProducesDifferentHash", "Covered", "tests/ndoc.rs::hash_changes_after_edit")
put(f, "ExtractState_EmptyInputsKeyOmittedForBareComponent_RoundTrips", "Covered", "src/fatfile/composed.rs::parse_state_bare_component_without_input_block_has_empty_inputs")

# test/FileSystemSeamTests.cs (8) -- IFileSystem abstraction-seam interaction tests.
f = "test/FileSystemSeamTests.cs"
for m in [
    "ComponentSchemaParser_LoadComponentsFromDirectory_ReadsOnlyViaSeam",
    "ComponentResolver_LoadForTemplate_UsesSeamForDirectoryProbeAndReads",
    "ComponentResolver_LoadForTemplate_MissingComponentsDir_ReturnsEmpty",
    "TemplateResolver_LoadTemplate_ReadsEntirelyViaSeam",
    "TemplateResolver_LoadTemplate_MissingFile_ThrowsWithoutTouchingDisk",
    "TemplateResolver_ResolveThemeCode_FindsByIdInParentOfComponentsDir_ViaSeam",
    "TemplateResolver_LoadComponentSources_EnumeratesViaSeam",
    "TemplateResolver_LoadComponentSources_MissingDirectory_ReturnsEmpty",
]:
    put(f, m, "Excluded", J_SEAM)

# test/ImageMergerTests.cs (7)
f = "test/ImageMergerTests.cs"
put(f, "MergeImages_DistinctNames_SucceedsMerge", "Covered", "src/authoring/doc_state.rs::embed_image_records_manifest_and_bytes")
put(f, "MergeImages_SameNameSameHash_Deduplicates", "Covered", "src/authoring/doc_state.rs::embed_image_is_idempotent_on_identical_content")
put(f, "MergeImages_SameNameDifferentHash_ThrowsWithDescriptiveMessage", "Covered", "src/authoring/doc_state.rs::embed_image_rejects_name_with_different_content")
put(f, "CombineWithDocIngested_MergesManifestsAndBytes", "Covered", "src/authoring/doc_state.rs::embed_image_dedupes_shared_content_across_names")
put(f, "CombineWithDocIngested_DocEntryWithSameHashAsTheme_DedupesBytes", "Covered", "src/authoring/doc_state.rs::embed_image_dedupes_shared_content_across_names")
put(f, "CombineWithDocIngested_EmptyDocIngested_ReturnsEquivalentResult", "Excluded", J_IMAGE_MERGER)
put(f, "MergeImages_ComponentReferencingThemeImageViaStateVariable_NoCollision", "Excluded", J_IMAGE_MERGER)

# test/ImageSectionParserTests.cs (9)
f = "test/ImageSectionParserTests.cs"
put(f, "ExtractImages_WithMultipleEntries_ReturnsAll", "Covered", "src/fatfile/composed.rs::extract_image_blobs_decodes_hash_keyed_base64")
put(f, "ExtractImages_NoSection_ReturnsEmpty", "Covered", "src/fatfile/composed.rs::extract_image_blobs_empty_when_no_section")
put(f, "ExtractImages_DuplicateHash_Throws", "Covered", "src/fatfile/composed.rs::extract_image_blobs_rejects_duplicate_hash")
put(f, "WriteImagesSection_ProducesFormattedOutput", "Covered", "src/fatfile/mod.rs::images_section_round_trips_bytes_by_hash")
put(f, "RoundTrip_WriteAndExtract_ProducesIdenticalBytes", "Covered", "src/fatfile/mod.rs::images_section_round_trips_bytes_by_hash")
put(f, "RoundTrip_LargeImage_WritesMultipleBase64Lines", "Covered", "src/fatfile/composed.rs::extract_image_blobs_decodes_payload_split_across_multiple_lines")
put(f, "WriteImagesSection_DeduplicatedByHash", "Covered", "src/fatfile/mod.rs::images_section_round_trips_bytes_by_hash")
put(f, "StripImagesSection_RemovesBlock", "Covered", "src/fatfile/mod.rs::images_section_skips_malformed_lines")
put(f, "StripImagesSection_NoSection_ReturnsContent", "Covered", "src/fatfile/mod.rs::empty_images_section_parses_to_empty_map")

# test/MarkdownToTypstConverterTests.cs (34)
f = "test/MarkdownToTypstConverterTests.cs"
put(f, "Convert_EmptyString_ReturnsEmptyString", "Covered", "src/markdown.rs::markdown_to_typst_empty")
put(f, "Convert_NullString_ReturnsEmptyString", "Excluded", J_NULLARG)
put(f, "Convert_WhitespaceOnly_ReturnsEmptyString", "Covered", "src/markdown.rs::markdown_to_typst_whitespace_only_is_empty")
put(f, "Convert_Heading1_ProducesTypstHeading", "Covered", "src/markdown.rs::markdown_to_typst_headings")
put(f, "Convert_Heading2_ProducesTypstHeading", "Covered", "src/markdown.rs::markdown_to_typst_headings")
put(f, "Convert_Heading3_ProducesTypstHeading", "Covered", "src/markdown.rs::markdown_to_typst_headings")
put(f, "Convert_Heading4_ProducesTypstHeading", "Covered", "src/markdown.rs::markdown_to_typst_headings")
put(f, "Convert_HeadingWithInlineFormatting_PreservesFormatting", "Covered", "src/markdown.rs::markdown_to_typst_heading_preserves_inline_formatting")
put(f, "Convert_BoldText_ProducesTypstBold", "Covered", "src/markdown.rs::markdown_to_typst_bold_italic")
put(f, "Convert_ItalicText_ProducesTypstItalic", "Covered", "src/markdown.rs::markdown_to_typst_bold_italic")
put(f, "Convert_ItalicWithUnderscore_ProducesTypstItalic", "Covered", "src/markdown.rs::markdown_to_typst_italic_with_underscore")
put(f, "Convert_BoldAndItalicMixed_ProducesCorrectTypst", "Covered", "src/markdown.rs::markdown_to_typst_bold_italic")
put(f, "Convert_NestedBoldInItalic_ProducesCorrectTypst", "Covered", "src/markdown.rs::markdown_to_typst_nested_bold_in_italic")
put(f, "Convert_Link_ProducesTypstLink", "Covered", "src/markdown.rs::markdown_to_typst_link")
put(f, "Convert_LinkWithFormattedText_ProducesTypstLink", "Covered", "src/markdown.rs::markdown_to_typst_link_with_formatted_text")
put(f, "Convert_Image_ProducesTypstImage", "Covered", "src/markdown.rs::markdown_to_typst_image")
put(f, "Convert_ImageWithPath_ProducesTypstImage", "Covered", "src/markdown.rs::markdown_to_typst_image_with_path")
put(f, "Convert_InlineCode_ProducesTypstRaw", "Covered", "src/markdown.rs::markdown_to_typst_code_inline")
put(f, "Convert_FencedCodeBlockWithLanguage_ProducesTypstCodeBlock", "Covered", "src/markdown.rs::markdown_to_typst_code_block_with_lang")
put(f, "Convert_FencedCodeBlockWithoutLanguage_ProducesTypstCodeBlock", "Covered", "src/markdown.rs::markdown_to_typst_code_block_no_lang")
put(f, "Convert_UnorderedList_ProducesTypstList", "Covered", "src/markdown.rs::markdown_to_typst_unordered_list")
put(f, "Convert_OrderedList_ProducesTypstEnumeration", "Covered", "src/markdown.rs::markdown_to_typst_ordered_list")
put(f, "Convert_NestedList_ProducesIndentedTypstList", "Covered", "src/markdown.rs::markdown_to_typst_nested_list")
put(f, "Convert_ListWithInlineFormatting_PreservesFormatting", "Covered", "src/markdown.rs::markdown_to_typst_list_preserves_inline_formatting")
put(f, "Convert_BlockQuote_ProducesTypstQuote", "Covered", "src/markdown.rs::markdown_to_typst_blockquote")
put(f, "Convert_MultilineBlockQuote_ProducesTypstQuote", "Covered", "src/markdown.rs::markdown_to_typst_multiline_blockquote")
put(f, "Convert_SimpleTable_ProducesTypstTable", "Covered", "tests/markdown.rs::snapshot_table")
put(f, "Convert_TableWithAlignment_IncludesAlignments", "Covered", "src/markdown.rs::markdown_to_typst_table_with_alignment")
put(f, "Convert_HorizontalRule_ProducesTypstLine", "Covered", "src/markdown.rs::markdown_to_typst_thematic_break")
put(f, "Convert_TypstSpecialCharacters_AreEscaped", "Covered", "src/markdown.rs::markdown_to_typst_escape")
put(f, "Convert_HashInText_IsEscaped", "Covered", "src/markdown.rs::markdown_to_typst_escape")
put(f, "Convert_MixedDocument_ProducesCorrectTypst", "Covered", "src/markdown.rs::markdown_to_typst_mixed_document")
put(f, "Convert_ParagraphWithMultipleInlineElements_PreservesAll", "Covered", "tests/markdown.rs::snapshot_paragraphs")
put(f, "Convert_PlainText_PassesThroughWithEscaping", "Covered", "src/markdown.rs::markdown_to_typst_escape")

# test/NodeIdGeneratorTests.cs (4)
f = "test/NodeIdGeneratorTests.cs"
put(f, "Mint_ProducesIdsOfTheExpectedForm", "Covered", "src/model.rs::mint_produces_type_dash_4hex_format")
put(f, "Mint_DoesNotCollideWithExistingIds", "Covered", "src/model.rs::mint_avoids_collision_with_existing_ids")
put(f, "Mint_ManyInvocations_ProducesUniqueIds", "Covered", "src/model.rs::mint_avoids_collision_with_existing_ids")
put(f, "CollectIds_WalksTreeRecursively", "Covered", "src/model.rs::node_ids_collects_nested_ids")

# test/PdfRenderPipelineTests.cs (2) -- DI default-registration guidance.
f = "test/PdfRenderPipelineTests.cs"
put(f, "DefaultRegistration_WhenResolvedAndInvoked_ThrowsWithRegistrationGuidance", "Excluded", J_DI)
put(f, "ConsumerRegisteredImplementation_ReplacesDefault", "Excluded", J_DI)

# test/PreviewRendererTests.cs (10)
f = "test/PreviewRendererTests.cs"
put(f, "RenderComponentPreview_ComposesAndCompiles", "Covered", "tests/cli.rs::preview_valid_md_exit_zero")
put(f, "RenderComponentPreview_WithInputValues_MergesWithDefaults", "Excluded", J_COMPONENT_PREVIEW)
put(f, "RenderComponentPreview_WithThemeCode_UsesProvidedTheme", "Excluded", J_COMPONENT_PREVIEW)
put(f, "RenderComponentPreview_NullSource_ThrowsArgumentNullException", "Excluded", J_NULLARG)
put(f, "RenderComponentPreview_NullSchema_ThrowsArgumentNullException", "Excluded", J_NULLARG)
put(f, "RenderDocumentPreview_ComposesAndCompiles", "Covered", "tests/cli.rs::preview_composed_document_exit_zero")
put(f, "RenderDocumentPreview_NullState_ThrowsArgumentNullException", "Excluded", J_NULLARG)
put(f, "RenderDocumentPreview_NullTheme_ThrowsArgumentNullException", "Excluded", J_NULLARG)
put(f, "RenderComponentPreview_CompilerThrows_PropagatesException", "Covered", "tests/cli.rs::preview_compile_failing_md_nonzero")
put(f, "RenderDocumentPreview_CompilerThrows_PropagatesException", "Covered", "tests/cli.rs::preview_invalid_input_nonzero")

# test/StateBlockChunkerTests.cs (8)
f = "test/StateBlockChunkerTests.cs"
put(f, "Chunk_ExtractsJsonPreludeAsFirstBalancedObject", "Covered", "src/fatfile/composed.rs::parse_state_reads_identity_and_global_inputs")
put(f, "Chunk_ExtractsDocumentInputBody", "Covered", "src/fatfile/composed.rs::parse_state_reads_identity_and_global_inputs")
put(f, "Chunk_ExtractsComponentInputsInAppearanceOrder", "Covered", "src/fatfile/composed.rs::parse_state_builds_node_tree_with_inputs")
put(f, "Chunk_ComponentInputJoinedId_IsComponentIdHyphenInstance", "Covered", "src/fatfile/composed.rs::parse_state_builds_node_tree_with_inputs")
put(f, "Chunk_AcceptsAttributesInEitherOrder", "Covered", "src/fatfile/composed.rs::attr_value_reads_key_regardless_of_attribute_order")
# The Rust chunker scans for the first </component-input> close and does not
# reject a nested block; rejection is a C#-only parser guard with no analogue.
put(f, "Chunk_RejectsNestedComponentInputBlocks", "Excluded", J_STATE_WRITER)
put(f, "Chunk_RejectsMissingDocumentInput", "Covered", "src/fatfile/composed.rs::parse_state_errors_on_malformed_prelude")
put(f, "Chunk_RejectsUnbalancedJson", "Covered", "src/fatfile/composed.rs::parse_state_errors_on_malformed_prelude")

# test/StateBlockWriterTests.cs (19)
f = "test/StateBlockWriterTests.cs"
put(f, "Write_JsonPrelude_ContainsTemplateAndThemeIds", "Covered", "src/authoring/doc_state.rs::snapshot_composed_state_fat_file")
put(f, "Write_JsonPrelude_ExcludesInputsFromNodes", "Excluded", J_STATE_WRITER)
put(f, "Write_JsonPrelude_PreservesTreeHierarchy", "Covered", "src/authoring/doc_state.rs::snapshot_composed_state_fat_file")
put(f, "Write_JsonPrelude_OmitsContentHashWhenNull", "Excluded", J_STATE_WRITER)
put(f, "Write_JsonPrelude_IncludesContentHashWhenSet", "Excluded", J_STATE_WRITER)
put(f, "Write_EmitsDocumentInputWithFlatKeyedGlobalInputs", "Covered", "src/authoring/doc_state.rs::snapshot_composed_state_fat_file")
put(f, "Write_DocumentInputFrontmatter_HasOnlyTemplateIdWhenGlobalInputsEmpty", "Excluded", J_STATE_WRITER)
put(f, "Write_EmitsComponentInputsWithSplitTagAttributes", "Covered", "src/authoring/doc_state.rs::snapshot_composed_state_fat_file")
put(f, "Write_EmitsComponentInputsInDfsOrder", "Excluded", J_STATE_WRITER)
put(f, "Write_ComponentInputFrontmatter_IsFlatKeyedMapWithoutComponentId", "Excluded", J_STATE_WRITER)
put(f, "Write_RoutesContentInputsToHtmlCommentDelimitedBlocks", "Excluded", J_STATE_WRITER)
put(f, "Write_EmptyFrontmatter_WhenNodeHasOnlyContentInputs", "Excluded", J_STATE_WRITER)
put(f, "Write_EmptyFrontmatter_WhenNodeHasNoInputsAtAll", "Excluded", J_STATE_WRITER)
put(f, "Write_DateTimeValues_EmittedAsIso8601", "Excluded", J_STATE_WRITER)
put(f, "Write_MultiLineStringValues_UseYamlBlockScalar", "Excluded", J_STATE_WRITER)
put(f, "Write_BooleanAndStringValues_UseYamlScalars", "Excluded", J_STATE_WRITER)
put(f, "Write_ThenChunk_YieldsAllExpectedPieces", "Covered", "src/authoring/doc_state.rs::write_then_read_round_trips_document")
put(f, "Write_RoundTripsByteIdenticallyOnSecondWrite", "Covered", "src/authoring/doc_state.rs::write_then_read_round_trips_document")
put(f, "Write_PreludeNodeId_MatchesJoinedComponentIdAndInstance", "Covered", "src/fatfile/composed.rs::parse_state_builds_node_tree_with_inputs")

# test/StateCanonicaliserTests.cs (8) -- canonical-hash input behaviour.
f = "test/StateCanonicaliserTests.cs"
put(f, "Canonicalise_ScalarInputOrdering_DoesNotAffectOutput", "Excluded", J_CANON_HASH)
put(f, "Canonicalise_GlobalInputOrdering_DoesNotAffectOutput", "Excluded", J_CANON_HASH)
put(f, "Canonicalise_WhitespaceEditsToContentMarkdown_DoNotAffectOutput", "Excluded", J_CANON_HASH)
put(f, "Canonicalise_RunsOfThreePlusBlankLines_CollapseToTwo", "Excluded", J_CANON_HASH)
put(f, "Canonicalise_ImagesSortedByName_RegardlessOfInputOrder", "Excluded", J_CANON_HASH)
put(f, "Canonicalise_OneWordEditToContent_ChangesOutput", "Excluded", J_CANON_HASH)
put(f, "Canonicalise_NodeReorder_ChangesOutput", "Excluded", J_CANON_HASH)
put(f, "Canonicalise_DropsContentHashFromPrelude", "Excluded", J_CANON_HASH)

# test/TaggedBlockParserTests.cs (9)
f = "test/TaggedBlockParserTests.cs"
put(f, "Parse_FlatKeyedYamlFrontmatter_DeserialisesAsRoot", "Covered", "src/fatfile/composed.rs::parse_state_builds_node_tree_with_inputs")
put(f, "Parse_HtmlCommentDelimitedContentBlocks_AreExtracted", "Covered", "src/fatfile/composed.rs::parse_content_blocks_extracts_named_blocks_in_order")
put(f, "Parse_MultipleDistinctContentBlocks_PreserveOrder", "Covered", "src/fatfile/composed.rs::parse_content_blocks_extracts_named_blocks_in_order")
put(f, "Parse_EmptyMarkdownSection_ReturnsNoContentBlocks", "Covered", "src/fatfile/composed.rs::parse_content_blocks_empty_body_yields_no_blocks")
put(f, "Parse_NormalisesCrlfLineEndings", "Covered", "src/fatfile/composed.rs::parse_tagged_block_normalises_crlf_line_endings")
put(f, "Parse_RejectsMissingOpeningFence", "Covered", "src/fatfile/composed.rs::parse_state_errors_on_malformed_prelude")
put(f, "Parse_RejectsMissingClosingFence", "Covered", "src/fatfile/composed.rs::parse_state_errors_on_malformed_prelude")
put(f, "Parse_RejectsUnmatchedStartMarker", "Covered", "src/fatfile/ndoc.rs::ndoc_document_parse_error_on_missing_end_marker")
# The Rust parse_content_blocks accepts repeated block names (last-write-wins at
# the input map); duplicate rejection is a C#-only parser guard, no analogue.
put(f, "Parse_RejectsDuplicateContentBlockName", "Excluded", J_STATE_WRITER)

# test/TemplateResolverTests.cs (23)
f = "test/TemplateResolverTests.cs"
put(f, "BuildDefaultState_GlobalInputsUseTemplateDefaultsWhenProvided", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "BuildDefaultState_FallsBackToTypeDefaultsWhenNoExplicitDefault", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "BuildDefaultState_CarriesTemplateIdentityOntoState", "Covered", "tests/cli.rs::e2e_doc_new")
put(f, "BuildDefaultState_NullDocumentInputs_ReturnsEmptyGlobalInputs", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "ResolveThemeCode_ExplicitPath_ReadsFile", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "ResolveThemeCode_ExplicitPathMissing_Throws", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "ResolveThemeCode_FindsByIdInParentOfComponentsDir", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "ResolveThemeCode_MatchIsCaseInsensitive", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "ResolveThemeCode_NotFound_Throws", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "LoadComponentSources_DirectoryAbsent_ReturnsEmpty", "Excluded", J_DIVERGE_FAILCLOSED)
put(f, "LoadComponentSources_ReturnsEveryNcmpFileKeyedByFileName", "Covered", "src/schema/parse.rs::load_components_from_dir_is_stable_order")
put(f, "ParserLoadComponentsFromDirectory_DuplicateComponentIds_Throws", "Covered", "src/schema/parse.rs::parse_component_file_names_path_on_failure")
put(f, "LoadTemplate_MissingFile_Throws", "Covered", "src/schema/parse.rs::missing_frontmatter_is_typed_error")
put(f, "LoadTemplate_WrongExtension_Throws", "Covered", "tests/cli.rs::e2e_render_rejects_bare_typ")
put(f, "LoadTemplate_ValidFile_ReturnsParsedSchema", "Covered", "src/schema/parse.rs::parse_template_reads_inputs_and_allowed_components")
put(f, "BuildDefaultState_NoDefaultLayout_NodesIsEmpty", "Covered", "tests/cli.rs::e2e_doc_new")
put(f, "BuildDefaultState_DefaultScaffold_MaterialisesLayout", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "BuildDefaultState_EmptyScaffold_KeepsNodesEmpty_KeepsGlobalInputs", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "BuildDefaultState_TwoCalls_ProduceNonCollidingIds", "Covered", "src/model.rs::mint_avoids_collision_with_existing_ids")
put(f, "BuildDefaultState_SchemaDefaultFlowsThrough_WhenNotSeeded", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "BuildDefaultState_TemplateSeededValueWinsOverComponentDefault", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "BuildDefaultState_LayoutOnlyInputKey_AppendedAfterSchemaDefaults", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "BuildDefaultState_RequiredInputCanBeOmitted_FromMaterialisedNode", "Excluded", J_TEMPLATE_RESOLVER)

# test/Templating/Catalogue/DocumentTemplateCatalogueGetDetailsTests.cs (2)
f = "test/Templating/Catalogue/DocumentTemplateCatalogueGetDetailsTests.cs"
put(f, "GetDetails_ReadsTemplateContentThroughInjectedFileSystem", "Excluded", J_SEAM)
put(f, "GetDetails_UnknownTemplateId_ReturnsNull", "Covered", "tests/cli.rs::e2e_template_show_unknown")

# test/Templating/ComponentSchemaMigrationTests.cs (10)
f = "test/Templating/ComponentSchemaMigrationTests.cs"
put(f, "ParseComponent_InputRequiredFalse_IsHonoured", "Covered", "src/schema/mod.rs::input_schema_serde_round_trip")
put(f, "ParseComponent_InputWithoutRequired_DefaultsToTrue", "Covered", "src/schema/parse.rs::parse_component_reads_id_and_inputs")
put(f, "ParseComponent_WithContentSection_ParsesEntries", "Covered", "src/schema/parse.rs::parse_component_content_section_folds_entries_with_required_defaulting_true")
put(f, "ParseComponent_ContentEntryRequiredFalse_IsHonoured", "Covered", "src/schema/parse.rs::parse_component_content_section_folds_entries_with_required_defaulting_true")
put(f, "ParseComponent_ContentEntryMissingDescription_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseComponent_DuplicateContentNames_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseComponent_WithRequiredSchema_ExposesField", "Excluded", J_SCHEMA_MODEL)
put(f, "ParseComponent_WithoutRequiredSchema_HasNullRequiredSchema", "Excluded", J_SCHEMA_MODEL)
put(f, "ComponentSchema_SchemaProperty_MatchesFromFile", "Covered", "src/schema/mod.rs::schema_round_trip_component")
put(f, "ComponentSchema_SchemaProperty_DoesNotIncludeRenderingOnlyFields", "Excluded", J_SCHEMA_MODEL)

# test/Templating/DocumentAuthoringServiceContextTests.cs (2) -- context override.
f = "test/Templating/DocumentAuthoringServiceContextTests.cs"
put(f, "AddNode_WithExplicitContextPointingAtOutOfFolderTemplate_Succeeds", "Excluded", J_SUP_DOCMISC)
put(f, "AddNode_ContextResolvesToBogusTemplatePath_ReturnsTemplateNotFound", "Excluded", J_SUP_DOCMISC)

# test/Templating/ItemCollectionLoaderTests.cs (6) -- dual-root (global+project) loader.
f = "test/Templating/ItemCollectionLoaderTests.cs"
put(f, "Load_MergesItemsFromBothRootsIntoSingleCollection", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Load_ProjectItemReplacesGlobalItemWithSameId_AndLogsInfo", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Load_MissingProjectRoot_LoadsFromGlobalOnly", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Load_InvalidItemSkippedWithWarning", "Covered", "tests/cli.rs::e2e_item_load")
put(f, "Load_GroupsItemsByDistinctCollection", "Covered", "src/item/mod.rs::summarise_groups_by_collection")
put(f, "Load_BothRootsMissing_ReturnsEmpty", "Excluded", J_ITEM_SUBSYSTEM)

# test/Templating/ItemCollectionTests.cs (9) -- collection lookups.
f = "test/Templating/ItemCollectionTests.cs"
put(f, "LookupById_ReturnsItem", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "LookupById_MissingId_ReturnsNull", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "LookupByTag_ReturnsAllItemsWithTag", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "LookupByTag_UnknownTag_ReturnsEmpty", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "LookupBySchema_ReturnsAllItemsWithThatSchema", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "LookupByInputKey_ReturnsItemsThatDeclareThatKey", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Schemas_ReturnsDistinctSchemaNames", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Tags_ReturnsUniqueTagsAcrossItems", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Items_PreservesInputOrder", "Covered", "src/item/mod.rs::parse_reads_reserved_keys_and_inputs")

# test/Templating/ItemNodeBuilderTests.cs (9)
f = "test/Templating/ItemNodeBuilderTests.cs"
put(f, "Build_ScalarOnlyItem_ReturnsLeafNode", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Build_ItemWithListInput_ReturnsParentWithNChildren", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Build_RecordFieldsRoutedToChildInputs", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Build_RecordFieldsRoutedToChildContent", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Build_ChildNodeIdsAreUnique", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Build_ParentSchemaNotFound_ReturnsError", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Build_NoAllowedChildren_WithListInput_ReturnsError", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Build_ZeroMatchingChild_ReturnsError", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Build_AmbiguousChildMatch_ReturnsError", "Excluded", J_ITEM_SUBSYSTEM)

# test/Templating/ItemParserTests.cs (19)
f = "test/Templating/ItemParserTests.cs"
put(f, "Parse_FullItem_PopulatesAllFields", "Covered", "src/item/mod.rs::parse_reads_reserved_keys_and_inputs")
put(f, "Parse_WithoutId_AcceptsItem", "Covered", "src/item/mod.rs::parse_item_without_id_is_accepted")
# Rust captures any non-reserved frontmatter key as a user input via serde
# flatten, including a stray `$`-prefixed key; C# rejects it. Divergence, no
# equivalent observable outcome.
put(f, "Parse_DollarPrefixedUserKey_Throws", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Parse_DuplicateContentBlockName_Throws", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Parse_UnmatchedStartMarker_Throws", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Parse_PreservesDeclaredInputKeyOrder", "Covered", "src/item/mod.rs::parse_reads_reserved_keys_and_inputs")
put(f, "Parse_WithoutTags_DefaultsToEmptyList", "Covered", "src/item/mod.rs::parse_item_without_tags_defaults_to_empty_list")
put(f, "Parse_MissingSchema_Throws", "Covered", "src/item/mod.rs::missing_required_reserved_key_is_typed_error")
put(f, "Parse_MissingCollection_Throws", "Covered", "src/item/mod.rs::missing_required_reserved_key_is_typed_error")
put(f, "ParseContent_SameFormatWorksForComponentInputBlockBody", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "ResolveImagePath_ResolvesAlongsideSourceFile", "Covered", "src/item/mod.rs::validate_resolves_present_image")
put(f, "Parse_IndexedBlockName_IsAccepted", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Parse_DuplicateIndexedBlockName_Throws", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Parse_BelowSentinel_MaterialisesIndexedBlocks", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Parse_BelowSentinel_GapInIndices_Throws", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Parse_YamlSequenceForm_MaterialisesDirectly", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Parse_ExampleScopeItem_ParsesAndMaterialisesPhases", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Parse_ExampleScopeItem_ValidatesAgainstSchema", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Parse_MixedScalarAndListInputs_BothPresent", "Excluded", J_ITEM_SUBSYSTEM)

# test/Templating/ItemValidatorTests.cs (15)
f = "test/Templating/ItemValidatorTests.cs"
put(f, "Validate_MissingRequiredInput_ReportsIssue", "Covered", "src/item/mod.rs::validate_reports_missing_required_input")
put(f, "Validate_MissingRequiredContent_ReportsIssue", "Covered", "src/item/mod.rs::validate_reports_missing_required_content")
# Rust ItemValidator checks required-presence and image resolvability only; it
# does not type-check scalar values or track content-block declarations, so
# type-mismatch / undeclared-content-block have no observable analogue.
put(f, "Validate_TypeMismatchOnNumber_ReportsIssue", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Validate_UndeclaredContentBlock_ReportsIssue", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Validate_UnknownSchema_ReportsSingleIssueAndStops", "Covered", "src/item/mod.rs::validate_reports_unknown_schema")
put(f, "Validate_ExtraInputsBeyondSchema_NoIssue", "Covered", "src/item/mod.rs::validate_ignores_inputs_beyond_schema")
put(f, "Validate_OptionalInputAndContentMissing_NoIssue", "Covered", "src/item/mod.rs::validate_passes_when_required_inputs_present")
put(f, "Validate_EveryIssueIncludesSourcePath", "Covered", "src/item/mod.rs::validate_issue_carries_source_path")
put(f, "Validate_MissingImageFile_ReportsIssue", "Covered", "src/item/mod.rs::validate_reports_missing_image")
put(f, "Validate_PresentImageFile_NoMissingImageIssue", "Covered", "src/item/mod.rs::validate_resolves_present_image")
put(f, "Validate_ValidListInput_NoIssues", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Validate_ListInput_MissingRequiredFieldAtIndex_ReportsIssue", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Validate_ListInput_UndeclaredField_ReportsIssue", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Validate_ListInput_WrongOuterShape_ReportsTypeMismatch", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Validate_ListInput_ImageFieldMissingFile_ReportsIssue", "Excluded", J_ITEM_SUBSYSTEM)

# test/Templating/LocalFolderResolutionContextTests.cs (8) -- path-resolution context.
f = "test/Templating/LocalFolderResolutionContextTests.cs"
for m in [
    "FromDocPath_ResolvesSiblingTemplateComponentsAndTheme",
    "FromDocPath_HonoursStateIdsAtResolveTime",
    "ExplicitTemplatePathOverride_ReplacesTemplateOnly_AndDerivesSiblingsFromOverrideFolder",
    "ExplicitComponentsDirectoryOverride_ReplacesOnlyComponents",
    "ExplicitThemePathOverride_ReplacesOnlyTheme",
    "AllThreeOverrides_ReplaceEveryInferredPath",
    "Resolution_ReturnsAbsolutePaths_ForRelativeInputs",
    "Resolution_ReturnsAbsoluteInferredPaths_WhenDocPathIsRelative",
]:
    put(f, m, "Excluded", J_SUP_DOCMISC)

# test/Templating/SchemaCatalogueTests.cs (6) -- dual-root schema catalogue.
f = "test/Templating/SchemaCatalogueTests.cs"
put(f, "Load_FromBothRoots_ExposesAllSchemas", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Load_ProjectShadowsGlobalByName_AndLogsWarning", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Load_MissingGlobalRoot_LoadsFromProjectOnly", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Load_MissingProjectRoot_LoadsFromGlobalOnly", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "Load_BothRootsMissing_LoadsNothing", "Covered", "src/schema/parse.rs::empty_dir_yields_zero_components")
put(f, "Lookup_UnknownName_ReturnsNull", "Covered", "src/schema/mod.rs::catalogue_lookup_unknown_returns_none")

# test/Templating/SchemaMatcherTests.cs (7) -- item<->schema structural matcher.
f = "test/Templating/SchemaMatcherTests.cs"
for m in [
    "Match_NoRequiredSchema_ItemSupersetMatches",
    "Match_RequiredSchema_ExcludesItemsWithDifferentSchema",
    "Match_ItemMissingRequiredInput_Excluded",
    "Match_ItemMissingRequiredContent_Excluded",
    "Match_OptionalInputAndContentMissing_Allowed",
    "Match_ItemWithExtraInputsAndContent_Included",
    "Match_SchemaNameFilterRunsBeforeStructuralFilter",
]:
    put(f, m, "Excluded", J_ITEM_SUBSYSTEM)

# test/Templating/SchemaParserTests.cs (18)
f = "test/Templating/SchemaParserTests.cs"
put(f, "FromFile_CompleteSchema_ParsesAllFields", "Covered", "src/schema/parse.rs::parse_component_reads_id_and_inputs")
put(f, "FromFile_RequiredFalse_IsHonoured", "Covered", "src/schema/mod.rs::input_schema_serde_round_trip")
put(f, "FromFile_MissingName_Throws", "Covered", "src/schema/parse.rs::parse_component_missing_component_id_is_typed_error")
put(f, "FromFile_MissingDescription_Throws", "Covered", "src/schema/parse.rs::parse_component_file_names_path_on_failure")
# Rust's serde-backed parser does not enforce duplicate-name or name-character
# rules (the FromFile schema-builder validations are C#-only); no analogue.
put(f, "FromFile_DuplicateInputNames_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "FromFile_DuplicateContentNames_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "FromFile_InvalidNameCharacters_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "FromFile_PreservesInputsAndContentOrder", "Covered", "src/schema/parse.rs::parse_component_reads_id_and_inputs")
put(f, "FromComponent_ProducesCanonicalSchema", "Covered", "src/schema/mod.rs::schema_round_trip_component")
put(f, "FromComponent_LegacyContentTypedInputs_BecomeContentEntries", "Covered", "src/schema/parse.rs::parse_component_content_typed_input_becomes_content_entry")
# Rust has a single parse path (no FromComponent vs FromFile distinction).
put(f, "FromComponentAndFromFile_ProduceSameShapeForEquivalentInputs", "Excluded", J_SCHEMA_MODEL)
put(f, "FromFile_ListInputWithValidFields_ParsesFieldsInOrder", "Excluded", J_SCHEMA_MODEL)
put(f, "FromFile_ListInputWithoutFields_Throws", "Covered", "src/schema/parse.rs::parse_component_list_input_kind_is_typed_error")
put(f, "FromFile_ListInputWithNestedList_Throws", "Covered", "src/schema/parse.rs::parse_component_list_input_kind_is_typed_error")
put(f, "FromFile_NonListInputWithFields_Throws", "Excluded", J_SCHEMA_MODEL)
put(f, "FromFile_ListInputWithDuplicateFieldNames_Throws", "Covered", "src/schema/parse.rs::parse_component_list_input_kind_is_typed_error")
put(f, "ParsedType_ListType_ReturnsList", "Covered", "src/schema/mod.rs::constraint_kind_serde_uses_snake_case")
put(f, "ComponentInputType_ContainsListMember", "Covered", "src/schema/mod.rs::constraint_kind_serde_uses_snake_case")

# test/Templating/SchemaTests.cs (8)
f = "test/Templating/SchemaTests.cs"
put(f, "Schema_DefaultInputsAndContent_AreEmptyLists", "Covered", "src/schema/mod.rs::component_schema_new_fields")
put(f, "Schema_PreservesInputAndContentOrder", "Covered", "src/schema/mod.rs::input_schema_serde_round_trip")
put(f, "SchemaInput_RequiredDefaultsToTrue", "Covered", "src/schema/parse.rs::parse_component_reads_id_and_inputs")
put(f, "SchemaInput_RequiredCanBeOverriddenToFalse", "Covered", "src/schema/mod.rs::input_schema_serde_round_trip")
put(f, "SchemaContent_RequiredDefaultsToTrue", "Covered", "src/schema/parse.rs::parse_component_content_section_folds_entries_with_required_defaulting_true")
put(f, "SchemaContent_RequiredCanBeOverriddenToFalse", "Covered", "src/schema/parse.rs::parse_component_content_section_folds_entries_with_required_defaulting_true")
put(f, "SchemaInput_ParsedType_ResolvesEachSupportedType", "Covered", "src/schema/mod.rs::constraint_kind_serde_uses_snake_case")
put(f, "SchemaInput_ParsedType_ThrowsForUnknownType", "Covered", "src/schema/parse.rs::unknown_input_kind_is_typed_error")

# test/TypstCompilerTests.cs (7)
f = "test/TypstCompilerTests.cs"
put(f, "CompileToPdf_SimpleContent_ReturnsPdfBytes", "Covered", "src/compiler.rs::compile_to_pdf_happy_path")
put(f, "CompileToPdf_WithVariables_ReturnsPdfBytes", "Covered", "src/compiler.rs::compile_to_pdf_happy_path")
put(f, "CompileToPdf_NullSource_ThrowsArgumentException", "Excluded", J_NULLARG)
put(f, "CompileToPdf_EmptySource_ThrowsArgumentException", "Covered", "src/compiler.rs::compile_to_pdf_empty_source_succeeds")
put(f, "CompileToPdf_WhitespaceSource_ThrowsArgumentException", "Excluded", J_DIVERGE_FAILCLOSED)
put(f, "CompileToPdf_InvalidSyntax_ThrowsInvalidOperationException", "Covered", "src/compiler.rs::compile_to_pdf_invalid_source")
put(f, "CompileToPdf_ImageOrPlaceholderWithEmptyName_RendersPlaceholder", "Covered", "src/compiler.rs::compile_to_pdf_image_or_placeholder_with_empty_name_renders_placeholder")

# ===========================================================================
# CLI AREA  (.reference/Typst/CLI/test/)
# ===========================================================================

# CLI/test/Commands/BuildCommandTests.cs (3)
f = "CLI/test/Commands/BuildCommandTests.cs"
put(f, "Execute_SuccessfulBuild_ReturnsZero", "Covered", "tests/cli.rs::e2e_build_produces_pdf")
put(f, "Execute_UpToDate_ReturnsZero", "Covered", "tests/cli.rs::ndoc_build_ndoc_typ")
put(f, "Execute_Error_ReturnsOne", "Covered", "tests/cli.rs::ndoc_build_malformed_doc")

# CLI/test/Commands/DocAddItemTests.cs (5)
f = "CLI/test/Commands/DocAddItemTests.cs"
put(f, "ExecuteAdd_WithItem_HappyPath_PassesItemInputsToAuthoringService", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "ExecuteAdd_ExplicitInputs_OverrideItemValues", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "ExecuteAdd_UnknownItemId_ExitsNonZero", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "ExecuteAdd_IncompatibleItem_ExitsNonZero", "Excluded", J_ITEM_SUBSYSTEM)
put(f, "ExecuteAdd_WithItem_UsesItemFolderAsImageBaseDirectory", "Excluded", J_ITEM_SUBSYSTEM)

# CLI/test/Commands/ImageCommandTests.cs (7)
f = "CLI/test/Commands/ImageCommandTests.cs"
put(f, "AddImage_ToDocument_UpdatesStateAndImagesSection", "Covered", "tests/cli.rs::e2e_image_add")
# Rust `image add` targets only `.ndoc.typ` documents; embedding into a
# component or theme file is not a supported CLI surface.
put(f, "AddImage_ToComponent_UpdatesFrontmatterAndImagesSection", "Excluded", J_COMPOSE_MODEL)
put(f, "AddImage_ToTheme_UpdatesFrontmatterAndImagesSection", "Excluded", J_COMPOSE_MODEL)
put(f, "AddImage_DuplicateName_Rejected", "Covered", "src/authoring/doc_state.rs::embed_image_rejects_name_with_different_content")
put(f, "AddImage_SameContentDifferentName_DeduplicatesBase64", "Covered", "tests/cli.rs::e2e_image_add_idempotent")
put(f, "AddImage_ImageNotFound_ReturnsError", "Covered", "tests/cli.rs::e2e_image_add_missing_image")
put(f, "AddImage_UnsupportedFileType_ReturnsError", "Covered", "tests/cli.rs::e2e_image_add_unsupported_target")

# CLI/test/Commands/ItemCommandTests.cs (12)
f = "CLI/test/Commands/ItemCommandTests.cs"
put(f, "ExecuteLoad_ValidItems_ExitsZero_AndPrintsCollectionSummary", "Covered", "tests/cli.rs::e2e_item_load")
# Rust `item load` fails closed on an unparseable item (typed error, non-zero)
# rather than skipping it with a warning and exiting zero -- divergent outcome.
put(f, "ExecuteLoad_InvalidItems_ExitsZero_AndNamesSkippedItems", "Excluded", J_DIVERGE_FAILCLOSED)
put(f, "ExecuteLoad_Json_EmitsCollectionsAndSkipped", "Covered", "tests/cli.rs::e2e_item_load_json")
put(f, "ExecuteLoad_MissingDirectory_ExitsNonZero", "Covered", "tests/cli.rs::e2e_item_missing_dir")
put(f, "ExecuteValidate_AllValid_ExitsZero", "Covered", "tests/cli.rs::e2e_item_validate_ok")
put(f, "ExecuteValidate_AnyFailure_ExitsNonZero_AndPrintsIssues", "Covered", "tests/cli.rs::e2e_item_validate_fail")
put(f, "ExecuteValidate_Json_EmitsIssuesArray", "Covered", "tests/cli.rs::e2e_item_validate_json")
put(f, "ExecuteSearch_MatchesTags", "Excluded", J_SUP_ITEMSEARCH)
put(f, "ExecuteSearch_FiltersByTag", "Excluded", J_SUP_ITEMSEARCH)
put(f, "ExecuteSearch_FiltersByCollection", "Excluded", J_SUP_ITEMSEARCH)
put(f, "ExecuteSearch_MatchesAcrossStringInputValues", "Excluded", J_SUP_ITEMSEARCH)
put(f, "ExecuteSearch_Json_ReturnsItemsArray", "Excluded", J_SUP_ITEMSEARCH)

# CLI/test/ComponentCommandTests.cs (21)
f = "CLI/test/ComponentCommandTests.cs"
put(f, "Create_ReturnsComponentCommandWithSubcommands", "Excluded", J_CMDTREE)
put(f, "ExecuteSchema_ValidComponent_EmitsCamelCaseJsonWithAllFields", "Covered", "tests/cli.rs::e2e_component_schema_json")
put(f, "ExecuteSchema_EmitsInputsWithFullFields", "Covered", "tests/cli.rs::e2e_component_schema_json")
put(f, "ExecuteSchema_NonexistentFile_ReturnsOneAndPrintsError", "Covered", "tests/cli.rs::e2e_component_schema_missing")
put(f, "ExecuteSchema_ParserThrows_ReturnsOneAndWritesErrorToStderr", "Covered", "tests/cli.rs::e2e_component_schema_missing")
put(f, "ExecuteSchema_StubFlag_EmitsNodeStubWithCorrectShape", "Excluded", J_CLI_STUB)
put(f, "ExecuteSchema_Stub_UsesDeclaredDefaultsWhenPresent", "Excluded", J_CLI_STUB)
put(f, "ExecuteSchema_ImageInput_EmitsTypeWidthHeightAndEmptyDefault", "Excluded", J_CLI_STUB)
put(f, "ExecuteSchema_StubForImageInput_UsesEmptyString", "Excluded", J_CLI_STUB)
put(f, "ExecuteSchema_Stub_UsesEmptyValuesForTypesWithoutDefaults", "Excluded", J_CLI_STUB)
put(f, "ExecuteSchema_Default_EmitsYamlFrontmatterShape", "Covered", "tests/cli.rs::e2e_component_schema")
put(f, "ExecuteSchema_StubDefault_EmitsComponentInputTaggedBlockWithContentContainers", "Excluded", J_CLI_STUB)
put(f, "ExecuteSchema_StubDefault_OmitsInputsKeyWhenNoScalarInputs", "Excluded", J_CLI_STUB)
put(f, "ExecuteList_DefaultTable_ListsComponentsAlphabetically", "Covered", "tests/cli.rs::e2e_component_list_stable_order")
put(f, "ExecuteList_DescriptionTruncatedToEightyCharactersWithEllipsis", "Excluded", J_CLI_STUB)
put(f, "ExecuteList_JsonFlag_EmitsMediumShapeArray", "Excluded", J_CLI_STUB)
put(f, "ExecuteList_JsonFullFlag_EmitsArrayOfFullSchemas", "Excluded", J_CLI_STUB)
put(f, "ExecuteList_FullWithoutJson_ReturnsOneAndErrors", "Excluded", J_CLI_STUB)
put(f, "ExecuteList_EmptyDirectory_ProducesEmptyTable", "Covered", "tests/cli.rs::e2e_component_list_empty_dir")
put(f, "ExecuteList_EmptyDirectory_JsonProducesEmptyArray", "Covered", "tests/cli.rs::e2e_component_list_empty_dir_json")
put(f, "ExecuteList_MissingDirectory_ReturnsOne", "Covered", "tests/cli.rs::e2e_component_list_missing_dir")

# CLI/test/ComposeCommandTests.cs (7) -- standalone `compose` command (folded into build).
f = "CLI/test/ComposeCommandTests.cs"
put(f, "Execute_MissingTemplate_ReturnsError", "Excluded", J_SUP_COMPOSE)
put(f, "Execute_WrongExtension_ReturnsError", "Excluded", J_SUP_COMPOSE)
put(f, "Execute_ValidTemplate_ComposesDocument", "Excluded", J_SUP_COMPOSE)
put(f, "Execute_WithInputsFile_ExtractsState", "Excluded", J_SUP_COMPOSE)
put(f, "Execute_InputsFileWithContentAfterState_WithoutForce_ReturnsError", "Excluded", J_SUP_COMPOSE)
put(f, "Execute_InputsFileWithContentAfterState_WithForce_Succeeds", "Excluded", J_SUP_COMPOSE)
put(f, "Execute_ComposerThrows_ReturnsError", "Excluded", J_SUP_COMPOSE)

# CLI/test/DocCommandBatchAddTests.cs (16) -- `doc batch-add` (not in v1 CLI).
f = "CLI/test/DocCommandBatchAddTests.cs"
put(f, "Create_IncludesBatchAddSubcommand", "Excluded", J_CMDTREE)
for m in [
    "ExecuteBatchAdd_InlineJsonLiteral_Parsed",
    "ExecuteBatchAdd_StdinInput_Parsed",
    "ExecuteBatchAdd_FilePathInput_Parsed",
    "ExecuteBatchAdd_MissingFilePath_ReturnsOne",
    "ExecuteBatchAdd_MalformedJson_ReturnsOneWithCleanError",
    "ExecuteBatchAdd_InvalidShape_MissingType_Rejects",
    "ExecuteBatchAdd_InvalidShape_MissingInputs_Rejects",
    "ExecuteBatchAdd_InvalidShape_MissingChildren_Rejects",
    "ExecuteBatchAdd_SingleRootNoChildren_BehavesLikeDocAdd",
    "ExecuteBatchAdd_TwoLevelSubtree_InsertsParentThenChildrenInOrder",
    "ExecuteBatchAdd_ValidationFailureMidSubtree_RollsBackEarlierInserts",
    "ExecuteBatchAdd_ParentId_ForwardedToAddNode",
    "ExecuteBatchAdd_BeforeId_ForwardedToAddNode",
    "ExecuteBatchAdd_AfterId_ForwardedToAddNode",
    "ExecuteBatchAdd_UnknownParentId_PropagatesNodeNotFoundErrorAndRollsBack",
]:
    put(f, m, "Excluded", J_SUP_BATCH)

# CLI/test/DocCommandNewSubcommandsTests.cs (11) -- list/details/set-many/patch/search.
f = "CLI/test/DocCommandNewSubcommandsTests.cs"
put(f, "Create_RegistersNewSubcommands", "Excluded", J_CMDTREE)
put(f, "List_JsonOutput_EmitsEnvelopeWithDocuments", "Excluded", J_SUP_DOCMISC)
put(f, "Details_NoNodeId_DelegatesToGetDocumentDetails", "Excluded", J_SUP_DOCMISC)
put(f, "Details_WithNodeId_DelegatesToGetComponentDetails", "Excluded", J_SUP_DOCMISC)
put(f, "SetMany_GlobalsRoute_DelegatesToSetGlobalInputs", "Excluded", J_SUP_DOCMISC)
put(f, "SetMany_NodeRoute_DelegatesToSetNodeInputs", "Excluded", J_SUP_DOCMISC)
put(f, "Patch_ParsesPatchesJsonAndDelegates", "Excluded", J_SUP_PATCH)
put(f, "Patch_OverlapErrorEnvelope_ExitsNonZero", "Excluded", J_SUP_PATCH)
put(f, "Search_JsonEnvelope", "Excluded", J_SUP_DOCMISC)
put(f, "Remove_WithoutWithChildren_PassesDeleteChildrenFalse", "Covered", "tests/cli.rs::e2e_doc_remove_preserves_children")
put(f, "Remove_NodeHasChildrenError_PrintsCode", "Covered", "tests/cli.rs::e2e_doc_remove_with_children")

# CLI/test/DocCommandOverrideFlagsTests.cs (8) -- per-document override flags.
f = "CLI/test/DocCommandOverrideFlagsTests.cs"
for m in [
    "ExecuteAdd_WithoutOverrides_UsesDocFolderResolution",
    "ExecuteAdd_WithTemplateOverride_ResolvesTemplateAndSiblingsFromOverridePath",
    "ExecuteAdd_WithoutTemplateOverride_FailsWhenDocFolderHasNoSiblings",
    "ExecuteAdd_WithExplicitComponentsDir_UsesThatDirectory",
    "ExecuteAdd_WithExplicitTheme_UsesThatThemeFile",
    "ExecuteSetGlobal_WithTemplateOverride_SucceedsWithoutDocFolderSiblings",
    "ExecuteSet_WithTemplateOverride_SucceedsWithoutDocFolderSiblings",
    "DocCommand_EachPerDocumentSubcommand_ExposesOverrideFlags",
]:
    put(f, m, "Excluded", J_SUP_DOCMISC)

# CLI/test/DocCommandTests.cs (26) -- mostly mock-delegation + command-tree.
f = "CLI/test/DocCommandTests.cs"
put(f, "Create_ReturnsDocCommandWithSubcommands", "Excluded", J_CMDTREE)
put(f, "Create_WithSchemaParser_IncludesSchemaSubcommand", "Excluded", J_CMDTREE)
put(f, "ExecuteNew_Success_CallsCreateDocumentAndPrintsOutput", "Covered", "tests/cli.rs::e2e_doc_new")
put(f, "ExecuteNew_Failure_PrintsErrorAndReturnsNonZero", "Covered", "tests/cli.rs::e2e_doc_new_unknown_template")
put(f, "ExecuteNew_WithoutOutputPath_PassesNullToService", "Covered", "tests/cli.rs::e2e_doc_new")
put(f, "ExecuteNew_WithScaffoldEmpty_PassesEmptyModeToService", "Excluded", J_MOCK)
# `ndoc new` takes only a template + optional output path; there is no
# `--scaffold` mode flag in the redesigned CLI to reject.
put(f, "ExecuteNew_WithInvalidScaffold_ReturnsErrorWithoutCallingService", "Excluded", J_SUP_DOCMISC)
put(f, "ExecuteOutline_Success_PrintsNodesWithIds", "Covered", "tests/cli.rs::e2e_doc_outline")
put(f, "ExecuteOutline_Failure_ReturnsNonZero", "Covered", "tests/cli.rs::e2e_doc_outline_missing")
put(f, "ExecuteAdd_ValidInputs_CallsAddNodeWithParsedJsonDictionary", "Covered", "tests/cli.rs::e2e_doc_add_under_parent_with_inputs")
# `ndoc doc add --inputs key=value` seeds string inputs; there is no JSON-inputs
# parsing surface, so "invalid JSON inputs" has no analogue.
put(f, "ExecuteAdd_InvalidJsonInputs_ReturnsErrorWithoutCallingService", "Excluded", J_SUP_DOCMISC)
put(f, "ExecuteAdd_NullInputs_PassesEmptyDictionary", "Covered", "tests/cli.rs::e2e_doc_add_at_root")
put(f, "ExecuteAdd_FailureFromService_ReturnsNonZero", "Covered", "tests/cli.rs::e2e_doc_add_unknown_type")
put(f, "ExecuteRemove_Success_CallsRemoveNode", "Covered", "tests/cli.rs::e2e_doc_remove_with_children")
put(f, "ExecuteRemove_NodeNotFound_ReturnsNonZero", "Covered", "tests/cli.rs::e2e_doc_remove_unknown")
put(f, "ExecuteSet_PlainStringValue_IsPassedAsString", "Covered", "tests/cli.rs::e2e_doc_set_node_input")
put(f, "ExecuteSet_JsonNumberLiteralValue_IsParsedAsNumber", "Covered", "tests/cli.rs::e2e_doc_set_document_input")
put(f, "ExecuteSet_JsonBoolLiteralValue_IsParsedAsBool", "Covered", "src/cli/mod.rs::coerce_boolean_value_parses_true_and_false_to_json_bool")
# The redesigned `doc set` coerces a single `--key/--value` against the declared
# kind; there is no `--set-image` flag, so its conflict/missing-equals paths
# have no analogue.
put(f, "ExecuteSet_SetImageAndKeyValue_Conflict_ReturnsNonZero", "Excluded", J_SUP_DOCMISC)
put(f, "ExecuteSet_SetImageOnly_CallsSetNodeInputWithFromFile", "Excluded", J_SUP_DOCMISC)
put(f, "ExecuteSet_SetImageMissingEquals_ReturnsNonZero", "Excluded", J_SUP_DOCMISC)
put(f, "ExecuteSet_NeitherKeyValueNorSetImage_ReturnsNonZero", "Covered", "tests/cli.rs::e2e_doc_set_requires_one_target")
put(f, "ExecuteSet_Failure_ReturnsNonZero", "Covered", "tests/cli.rs::e2e_doc_set_unknown_node")
put(f, "ExecuteSetGlobal_Success_CallsSetGlobalInput", "Covered", "tests/cli.rs::e2e_doc_set_document_input")
put(f, "ExecuteSetGlobal_NumericLiteral_CoercedToNumber", "Covered", "tests/cli.rs::e2e_doc_set_document_input")
put(f, "ExecuteSetGlobal_Failure_ReturnsNonZero", "Covered", "tests/cli.rs::e2e_doc_set_unknown_key")

# CLI/test/DocSchemaCommandTests.cs (10)
f = "CLI/test/DocSchemaCommandTests.cs"
put(f, "ExecuteSchema_Default_EmitsYamlFrontmatterWithTemplateFields", "Covered", "tests/cli.rs::e2e_doc_schema_template")
put(f, "ExecuteSchema_JsonFlag_EmitsCamelCaseJsonDocumentTemplateSchema", "Covered", "tests/cli.rs::e2e_doc_schema_template")
put(f, "ExecuteSchema_StubDefault_EmitsDocumentInputTaggedBlock", "Excluded", J_CLI_STUB)
put(f, "ExecuteSchema_StubDefault_UsesDeclaredDefaultsWhenPresent", "Excluded", J_CLI_STUB)
put(f, "ExecuteSchema_StubDefault_OmitsInputsKeyWhenTemplateHasNoInputs", "Excluded", J_CLI_STUB)
put(f, "ExecuteSchema_StubJsonFlag_EmitsLegacyJsonShape", "Excluded", J_SUP_DOCMISC)
put(f, "ExecuteSchema_StubJsonFlag_UsesEmptyValuesForTypesWithoutDefaults", "Excluded", J_SUP_DOCMISC)
put(f, "ExecuteSchema_NonexistentFile_ReturnsOneAndPrintsError", "Covered", "tests/cli.rs::e2e_doc_schema_missing")
put(f, "ExecuteSchema_ParserThrows_ReturnsOneAndWritesErrorToStderr", "Covered", "tests/cli.rs::e2e_doc_schema_missing")
put(f, "ExecuteSchema_ParserReturnsNull_ReturnsOneWithErrorMessage", "Covered", "tests/cli.rs::e2e_doc_schema_missing")

# CLI/test/InteractivePromptServiceTests.cs (9) -- interactive console prompting.
f = "CLI/test/InteractivePromptServiceTests.cs"
for m in [
    "PromptForInputs_StringInput_PromptsAndReturnsValue",
    "PromptForInputs_MultipleInputs_PromptsEach",
    "PromptForInputs_ContentInput_ReadsMultipleLines",
    "PromptForInputs_WithDefault_UsesDefaultOnEmptyInput",
    "PromptForInputs_WithDefault_UsesProvidedValue",
    "PromptForInputs_BooleanInput_ParsesValue",
    "PromptForInputs_ColorInput_ReturnsString",
    "PromptForInputs_DerivedLabel_FormatsHyphenatedName",
    "PromptForInputs_CustomLabel_UsesLabel",
]:
    put(f, m, "Excluded", J_PROMPT)

# CLI/test/PreviewCommandTests.cs (5)
f = "CLI/test/PreviewCommandTests.cs"
put(f, "Execute_DocumentFile_RendersAndOpens", "Covered", "tests/cli.rs::preview_valid_ndoc_typ_exit_zero")
put(f, "Execute_MissingFile_ReturnsError", "Covered", "tests/cli.rs::preview_invalid_input_nonzero")
put(f, "Execute_NullOpener_DoesNotThrow", "Excluded", J_MOCK)
put(f, "Execute_DocumentFile_NoBuildFalse_CallsBuild", "Excluded", J_MOCK)
put(f, "Execute_DocumentFile_NoBuildTrue_SkipsBuild", "Excluded", J_MOCK)

# CLI/test/ProgramTests.cs (4) -- root-command construction / global wiring.
f = "CLI/test/ProgramTests.cs"
put(f, "BuildRootCommand_RegistersAllCommands", "Covered", "tests/cli.rs::help_lists_commands")
put(f, "BuildRootCommand_DocCommandHasAllSubcommands", "Excluded", J_CMDTREE)
put(f, "BuildRootCommand_HasDescription", "Excluded", J_CMDTREE)
put(f, "RootCommand_ValidateWithNonexistentFile_ReturnsOne", "Covered", "tests/cli.rs::validate_invalid_ndoc_file")

# CLI/test/RenderCommandTests.cs (14)
f = "CLI/test/RenderCommandTests.cs"
put(f, "Execute_MissingFile_ReturnsError", "Covered", "tests/cli.rs::e2e_render_missing_input")
put(f, "Execute_UnrecognisedExtension_ReturnsError", "Covered", "tests/cli.rs::e2e_render_rejects_bare_typ")
put(f, "Execute_DocumentFile_CompilesToPdf", "Covered", "tests/cli.rs::e2e_render_produces_pdf")
put(f, "Execute_ComponentFile_PromptsAndRenders", "Excluded", J_PROMPT)
put(f, "Execute_ComponentFile_NoInputs_SkipsPrompting", "Excluded", J_PROMPT)
put(f, "Execute_CompilerThrows_ReturnsError", "Covered", "tests/cli.rs::preview_compile_failing_md_nonzero")
put(f, "Execute_DocumentFile_NoBuildFalse_CallsBuild", "Excluded", J_MOCK)
put(f, "Execute_DocumentFile_NoBuildTrue_SkipsBuild", "Excluded", J_MOCK)
put(f, "Execute_ComponentFile_NoBuildFalse_DoesNotCallBuild", "Excluded", J_MOCK)
put(f, "Execute_DocumentFile_NoOutputPath_WritesNextToSource", "Covered", "tests/cli.rs::e2e_render_produces_pdf")
put(f, "Execute_ComponentFile_NoOutputPath_WritesNextToSource", "Covered", "tests/cli.rs::e2e_render_component_default_output_next_to_source")
put(f, "Execute_WithExplicitOutputPath_WritesToThatPath", "Covered", "tests/cli.rs::e2e_render_output_override")
put(f, "Execute_DefaultOutputPath_OverwritesExistingFile", "Covered", "tests/cli.rs::e2e_render_default_output_overwrites_existing_file")
put(f, "Execute_DocumentFile_BuildRebuilds_CompilesFromTemp", "Covered", "tests/cli.rs::e2e_build_produces_pdf")

# CLI/test/SchemaFirstAuthoringEndToEndTests.cs (4)
f = "CLI/test/SchemaFirstAuthoringEndToEndTests.cs"
put(f, "SchemaStub_ThenBatchAdd_InsertsNodeWithExpectedInputs", "Excluded", J_SUP_BATCH)
put(f, "SchemaStub_WithUnknownInput_ValidateFailsWithCorrectError", "Covered", "tests/cli.rs::validate_invalid_ndoc_file")
put(f, "BatchAdd_JsonFileWithRelativeImagePath_ResolvesAgainstJsonDirectory", "Excluded", J_SUP_BATCH)
put(f, "BatchAdd_ThenBuild_PreservesDocIngestedImage", "Excluded", J_SUP_BATCH)

# CLI/test/Services/BuildServicePdfRenderPipelineTests.cs (3) -- DI pipeline wiring.
f = "CLI/test/Services/BuildServicePdfRenderPipelineTests.cs"
put(f, "RenderDocumentToPdf_BuildsAndCompilesTempFile_ReturnsBytes", "Covered", "tests/cli.rs::e2e_build_produces_pdf")
put(f, "RenderDocumentToPdf_ForwardsContextToBuildService", "Excluded", J_MOCK)
put(f, "RenderDocumentToPdf_BuildFailed_Throws", "Covered", "tests/cli.rs::ndoc_build_malformed_doc")

# CLI/test/Services/BuildServiceTests.cs (14)
f = "CLI/test/Services/BuildServiceTests.cs"
put(f, "Build_HashMatches_SkipsRecomposition", "Covered", "tests/cli.rs::ndoc_build_ndoc_typ")
put(f, "Build_CosmeticEditKeepsCanonicalHash_SkipsRebuild", "Excluded", J_BUILD_SERVICE)
put(f, "Build_HashDiffers_Recomposes", "Covered", "tests/cli.rs::ndoc_build_ndoc_typ")
put(f, "Build_NullHash_Recomposes", "Covered", "tests/cli.rs::e2e_build_produces_pdf")
put(f, "Build_MissingFile_ReturnsError", "Covered", "tests/cli.rs::e2e_build_missing_file")
put(f, "Build_WrongExtension_ReturnsError", "Covered", "tests/cli.rs::e2e_build_unsupported_extension")
put(f, "Build_MissingTemplate_ReturnsError", "Covered", "tests/cli.rs::ndoc_build_malformed_doc")
# The Rust port has no theme concept, so a build cannot fail on a missing theme.
put(f, "Build_MissingTheme_ReturnsError", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "Build_ResultIncludesTempFilePath", "Excluded", J_MOCK)
put(f, "Build_PreservesDocIngestedImages_InStateManifest", "Covered", "tests/cli.rs::e2e_build_composed_document_with_embedded_image")
put(f, "Build_PreservesDocIngestedImages_InComposedBytes", "Covered", "tests/cli.rs::e2e_build_composed_document_with_embedded_image")
put(f, "Build_UpToDate_PreservesDocIngestedImagesInMemoryState", "Excluded", J_BUILD_SERVICE)
put(f, "Build_WithResolutionContext_UsesContextPaths_NotDocFolder", "Excluded", J_SUP_DOCMISC)
put(f, "Build_UpToDate_StillPrepareTempAndExtractImages", "Excluded", J_BUILD_SERVICE)

# CLI/test/Services/JsonOutputTests.cs (3) -- camelCase/indented JSON serializer options.
f = "CLI/test/Services/JsonOutputTests.cs"
put(f, "Options_SerialisesComponentSchema_WithCamelCasePropertyNames", "Covered", "tests/cli.rs::e2e_component_schema_json")
put(f, "Options_SerialisesComponentSchema_WithIndentedOutput", "Covered", "tests/cli.rs::doc_json_envelope")
# The Rust ComponentSchema has no optional/nullable metadata fields (label,
# default, width, height, description), so there are no explicit nulls to emit.
put(f, "Options_SerialisesComponentSchema_WithExplicitNullsForOptionalFields", "Excluded", J_SCHEMA_MODEL)

# CLI/test/Services/TempBuildServiceTests.cs (7) -- temp-dir prep + image extraction.
f = "CLI/test/Services/TempBuildServiceTests.cs"
put(f, "PrepareTempDirectory_CreatesTempDirAndCopiesFile", "Covered", "tests/cli.rs::e2e_build_produces_pdf")
put(f, "PrepareTempDirectory_StablePath_SameSourceProducesSameDir", "Excluded", J_MOCK)
put(f, "ExtractImages_FirstBuild_DecodesAllImages", "Covered", "tests/cli.rs::e2e_build_composed_document_with_embedded_image")
put(f, "ExtractImages_NoChanges_SkipsDecoding", "Excluded", J_BUILD_SERVICE)
put(f, "ExtractImages_ChangedImage_ReDecodes", "Excluded", J_BUILD_SERVICE)
put(f, "ExtractImages_RemovedImage_CleansUp", "Excluded", J_BUILD_SERVICE)
put(f, "ExtractImages_MultipleManifestEntriesSameHash_CreatesMultipleFiles", "Covered", "src/authoring/doc_state.rs::embed_image_dedupes_shared_content_across_names")

# CLI/test/TemplateCommandTests.cs (5)
f = "CLI/test/TemplateCommandTests.cs"
put(f, "Show_WithLayout_RendersHeaderAndAsciiTree", "Covered", "tests/cli.rs::e2e_template_show")
# `template show` has no defaultLayout/inputs-seeding flag (no defaultLayout in
# the Rust template model), so inline seeded layout values have no analogue.
put(f, "Show_WithLayoutAndInputsFlag_IncludesInlineSeededValues", "Excluded", J_TEMPLATE_RESOLVER)
put(f, "Show_WithoutLayout_RendersDefaultLayoutNone", "Covered", "tests/cli.rs::e2e_template_show")
put(f, "Show_UnknownTemplateId_ExitsNonZeroAndReportsError", "Covered", "tests/cli.rs::e2e_template_show_unknown")
put(f, "Show_JsonFlag_EmitsStructuredEnvelope", "Covered", "tests/cli.rs::e2e_template_show_json")

# CLI/test/ValidateCommandTests.cs (27)
f = "CLI/test/ValidateCommandTests.cs"
put(f, "DetectFileType_ComponentFile_ReturnsComponent", "Excluded", J_CLI_VALIDATE_SUBSET)
put(f, "DetectFileType_DocumentTemplateFile_ReturnsDocumentTemplate", "Excluded", J_CLI_VALIDATE_SUBSET)
put(f, "DetectFileType_ThemeFile_ReturnsTheme", "Excluded", J_CLI_VALIDATE_SUBSET)
put(f, "DetectFileType_DocumentFile_ReturnsDocument", "Covered", "tests/cli.rs::validate_composed_document_exit_zero")
put(f, "DetectFileType_JsonFile_ReturnsNodeJson", "Excluded", J_SUP_DOCMISC)
put(f, "DetectFileType_UnknownExtension_ReturnsNull", "Covered", "tests/cli.rs::validate_unsupported_extension")
put(f, "Execute_MissingFile_ReturnsError", "Covered", "tests/cli.rs::validate_invalid_ndoc_file")
put(f, "Execute_UnrecognisedExtension_ReturnsError", "Covered", "tests/cli.rs::validate_unsupported_extension")
put(f, "Execute_ValidComponent_ReportsStructure", "Excluded", J_CLI_VALIDATE_SUBSET)
put(f, "Execute_ValidDocumentTemplate_ReportsStructure", "Excluded", J_CLI_VALIDATE_SUBSET)
put(f, "Execute_ValidTheme_ReportsStructure", "Excluded", J_CLI_VALIDATE_SUBSET)
put(f, "Execute_ValidDocument_DeepValidationReportsStructureAndPasses", "Covered", "tests/cli.rs::validate_composed_document_exit_zero")
# The Rust validator checks the images section parses/decodes and runs schema
# checks, but does not flag a node input that names an unregistered image; that
# specific dangling-name check has no analogue.
put(f, "Execute_DocumentWithDanglingImageName_FlagsImageNotRegisteredWithNodeId", "Excluded", J_CLI_VALIDATE_SUBSET)
put(f, "Execute_DocumentWithValidImageName_DoesNotFlag", "Covered", "tests/cli.rs::validate_composed_document_exit_zero")
put(f, "Execute_DocumentWithUnknownInput_ErrorMessageReferencesNodeId", "Covered", "tests/cli.rs::validate_composed_schema_error_exit_nonzero")
put(f, "Execute_ParserThrows_ReturnsError", "Covered", "tests/cli.rs::validate_invalid_ndoc_file")
put(f, "Execute_BothTemplateAndDocFlags_Rejected", "Excluded", J_SUP_DOCMISC)
put(f, "Execute_NodeJsonWithoutContext_Rejected", "Excluded", J_SUP_DOCMISC)
put(f, "Execute_DocumentWithDocFlag_Rejected", "Excluded", J_SUP_DOCMISC)
put(f, "Execute_ComponentWithTemplateFlag_Rejected", "Excluded", J_SUP_DOCMISC)
put(f, "Execute_NodeJson_ValidAgainstTemplate_Passes", "Excluded", J_SUP_DOCMISC)
put(f, "Execute_NodeJson_UnknownInput_CaughtWithPath", "Excluded", J_SUP_DOCMISC)
put(f, "Execute_NodeJson_MissingRequiredInput_Caught", "Excluded", J_SUP_DOCMISC)
put(f, "Execute_NodeJson_NestedChildError_PathAnnotated", "Excluded", J_SUP_DOCMISC)
put(f, "Execute_NodeJson_MalformedJson_ReturnsError", "Excluded", J_SUP_DOCMISC)
put(f, "Execute_DocumentWithSchemaViolation_DeepValidationFails", "Covered", "tests/cli.rs::validate_composed_schema_error_exit_nonzero")
put(f, "Execute_DocumentWithTemplateOverride_UsesAllowedComponents", "Excluded", J_SUP_DOCMISC)


# ---------------------------------------------------------------------------
# Validation + reporting
# ---------------------------------------------------------------------------

def main() -> None:
    write = "--write" in sys.argv

    # 1. Every in-scope case must be dispositioned exactly once.
    missing = []
    extra = []
    inscope_keys = set()
    for area in ("core", "cli"):
        for file, methods in inv[area].items():
            for entry in methods:
                key = f"{file}::{entry['method']}"
                inscope_keys.add(key)
                if key not in D:
                    missing.append(key)
    for key in D:
        if key not in inscope_keys:
            extra.append(key)
    if missing:
        raise SystemExit(f"{len(missing)} undispositioned in-scope cases, e.g. {missing[:5]}")
    if extra:
        raise SystemExit(f"{len(extra)} dispositions key cases not in inventory, e.g. {extra[:5]}")

    # 2. Roll-up per area.
    def rollup(area):
        c = g = e = 0
        for file, methods in inv[area].items():
            for entry in methods:
                disp = D[f"{file}::{entry['method']}"][0]
                if disp == "Covered":
                    c += 1
                elif disp == "Gap":
                    g += 1
                elif disp == "Excluded":
                    e += 1
                else:
                    raise SystemExit(f"bad disposition {disp}")
        return c, g, e

    core_c, core_g, core_e = rollup("core")
    cli_c, cli_g, cli_e = rollup("cli")
    print(f"Core: Covered={core_c} Gap={core_g} Excluded={core_e} sum={core_c+core_g+core_e} (expect 437)")
    print(f"CLI : Covered={cli_c} Gap={cli_g} Excluded={cli_e} sum={cli_c+cli_g+cli_e} (expect 221)")
    assert core_c + core_g + core_e == 437, "core sum mismatch"
    assert cli_c + cli_g + cli_e == 221, "cli sum mismatch"
    print(f"TOTAL in-scope Gaps (T4 input): {core_g + cli_g}")

    if write:
        stamp()
        print("Stamped dispositions into docs/reference-parity-map.md")


# Map row regex: | N | `Method` | behavior | disp | ptr |
ROW = re.compile(r"^\|\s*(\d+)\s*\|\s*`([^`]+)`\s*\|(.*)\|([^|]*)\|([^|]*)\|\s*$")
HEADER = re.compile(r"^#### `([^`]+)` \(\d+ cases\)\s*$")


def stamp() -> None:
    lines = MAP.read_text().splitlines()
    cur_file = None
    out = []
    for line in lines:
        h = HEADER.match(line)
        if h:
            cur_file = h.group(1)
            out.append(line)
            continue
        m = ROW.match(line)
        if m and cur_file and not line.strip().startswith("| # ") and "---" not in line:
            num, method, behavior = m.group(1), m.group(2), m.group(3)
            key = f"{cur_file}::{method}"
            if key in D:
                disp, ptr = D[key]
                if ptr and disp != "Excluded":
                    ptrcell = f" `{ptr}` "
                elif disp == "Excluded":
                    ptrcell = f" {ptr} "
                else:
                    ptrcell = "  "
                out.append(f"| {num} | `{method}` |{behavior}| {disp} |{ptrcell}|")
                continue
        out.append(line)
    MAP.write_text("\n".join(out) + "\n")


if __name__ == "__main__":
    main()
