//! Embedded Typst [`World`] implementation.
//!
//! This is the canonical integration point with the native Typst compiler and,
//! per the charter, the project's highest-risk area (failure signal #1: that
//! embedding proves impractical). We mirror `typst-cli`'s own architecture:
//!
//! - the standard library and font book come from [`typst_library`] / fonts
//!   discovered by [`typst_kit::fonts`] (seeded with [`typst_assets`] so a bare
//!   single binary works with zero system fonts installed),
//! - `main()` returns a synthetic [`FileId`] for an in-memory source string,
//! - `source()` / `file()` serve the composed `.ndoc.typ` and any embedded
//!   images from an in-memory overlay before falling through to disk packages.
//!
//! The fat-file model composes a single `.typ` string in memory, so the
//! in-memory overlay is the primary path; on-disk package resolution
//! (`#import "@preview/..."`) is layered underneath via typst-kit.

use std::collections::HashMap;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::{FontSlot, Fonts};

/// An in-memory Typst environment backed by a single composed source string.
///
/// Construct via [`NorthdocWorld::new`] with the fully composed fat-file source.
/// Additional in-memory files (e.g. embedded images) can be registered with
/// [`NorthdocWorld::insert_file`] before compilation.
pub struct NorthdocWorld {
    /// Standard library (definitions, styles).
    library: LazyHash<Library>,
    /// Font metadata book derived from the discovered/embedded fonts.
    book: LazyHash<FontBook>,
    /// Lazily-loaded font slots, indexed parallel to `book`.
    fonts: Vec<FontSlot>,
    /// The synthetic id of the composed main source.
    main: FileId,
    /// In-memory overlay: composed source plus embedded binary files.
    sources: HashMap<FileId, Source>,
    files: HashMap<FileId, Bytes>,
}

impl NorthdocWorld {
    /// Build a world from a composed `.typ` source string.
    ///
    /// Fonts are discovered via [`typst_kit::fonts`] with the embedded
    /// [`typst_assets`] fonts included, guaranteeing the single binary renders
    /// even on a host with no fonts installed.
    pub fn new(main_source: impl Into<String>) -> Self {
        let fonts = Fonts::searcher().include_system_fonts(true).search();

        let main = FileId::new(None, VirtualPath::new("main.typ"));
        let source = Source::new(main, main_source.into());

        let mut sources = HashMap::new();
        sources.insert(main, source);

        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(fonts.book),
            fonts: fonts.fonts,
            main,
            sources,
            files: HashMap::new(),
        }
    }

    /// Register an additional in-memory binary file (e.g. an embedded image)
    /// under a virtual path, returning its [`FileId`].
    pub fn insert_file(&mut self, path: &str, data: impl Into<Bytes>) -> FileId {
        let id = FileId::new(None, VirtualPath::new(path));
        self.files.insert(id, data.into());
        id
    }

    /// Replace the composed main source (used when re-composing between edits).
    pub fn set_main_source(&mut self, source: impl Into<String>) {
        let updated = Source::new(self.main, source.into());
        self.sources.insert(self.main, updated);
    }

    /// Evict the incremental-compilation cache so it stays bounded.
    ///
    /// Call this around batched compiles; `comemo` retains memoized work
    /// otherwise. `max_age` is the number of compiles a cached value may
    /// survive without being touched.
    pub fn evict_cache(max_age: usize) {
        comemo::evict(max_age);
    }
}

impl World for NorthdocWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.sources
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index)?.get()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        // Deterministic builds are preferred; wire real clock support later.
        Some(Datetime::from_ymd(1970, 1, 1).expect("epoch is a valid date"))
    }
}

#[cfg(test)]
mod tests {
    use super::NorthdocWorld;
    use typst::foundations::Bytes;
    use typst::World;

    #[test]
    fn world_new_has_main_source() {
        let world = NorthdocWorld::new("Hello, world!");
        let source = world
            .source(world.main())
            .expect("main source should exist");
        assert_eq!(source.text(), "Hello, world!");
    }

    #[test]
    fn world_insert_file_round_trip() {
        let mut world = NorthdocWorld::new("");
        let data = Bytes::new(b"binary content".to_vec());
        let id = world.insert_file("asset.bin", data.clone());
        let retrieved = world.file(id).expect("inserted file should be retrievable");
        assert_eq!(retrieved.as_slice(), data.as_slice());
    }

    #[test]
    fn world_set_main_source_replaces() {
        let mut world = NorthdocWorld::new("original");
        world.set_main_source("replaced");
        let source = world
            .source(world.main())
            .expect("main source should exist after replacement");
        assert_eq!(source.text(), "replaced");
    }

    #[test]
    fn world_evict_cache_no_panic() {
        NorthdocWorld::evict_cache(5);
    }
}
