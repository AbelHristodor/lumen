# Refactor: clean core + GUI split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Behavior-preserving refactor: smaller cache, faster index and search, zip-based hymn sync without libgit2, `main.rs` split by feature with one shared state.

**Architecture:** Core stays a pure-logic crate. `sync.rs` moves from git clone to HEAD-ETag plus GET of `hymns.zip`. GUI splits into per-tab modules that all take `&App`, a Clone struct of `Rc<RefCell<..>>` handles. Workers build the `Searcher` off thread and send one `Msg` enum over one channel.

**Tech Stack:** Rust 1.94, Slint 1.8, nucleo-matcher 0.3, ureq 2, zip 2, bincode 1.

**Spec:** `docs/superpowers/specs/2026-09-26-refactor-clean-core-and-gui-design.md`

## Global Constraints

- No new dependencies. Remove `git2` and `dirs`.
- Every existing feature and UI string stays.
- `cargo test -p hymnal-core` green after every task. `cargo clippy` clean.
- Commit after every task with a conventional-commit message.

---

### Task 1: Drop `body`, delete `build_index`, HashMap cache lookup

**Files:**
- Modify: `crates/hymnal-core/src/model.rs`, `pptx.rs`, `index.rs`, `search.rs`
- Test: `tests/pptx_test.rs`, `tests/index_test.rs`, `tests/search_test.rs`

**Interfaces:**
- Produces: `HymnEntry { number, title, slides, path, library, mtime }`. `ParsedPptx { number, title, slides }`. `index::refresh_index(root, lib, cached)` is the only indexer. `CACHE_VERSION = 7`.

- [ ] Replace `build_index(root, lib)` in tests with `refresh_index(root, lib, &[])`. Replace `parsed.body` asserts with `parsed.slides.join("\n")`. Remove `body:` from `HymnEntry` literals in tests.
- [ ] Run `cargo test -p hymnal-core`. Expect compile failures only from removed field.
- [ ] Remove `body` from `HymnEntry`, `ParsedPptx`, `extract`. Delete `build_index`. In `refresh_index` build `HashMap<&Path, &HymnEntry>` from `cached`. Bump `CACHE_VERSION` to 7. `eprintln!` to `log::warn!`. In `search.rs` derive body via `fold(&entry.slides.join("\n"))`.
- [ ] Run tests. Expect all pass.
- [ ] Commit: `refactor(core): drop duplicated body field, single indexer, O(1) cache lookup`

### Task 2: Parallel cold index

**Files:**
- Modify: `crates/hymnal-core/src/index.rs`
- Test: `tests/index_test.rs`

**Interfaces:**
- Produces: `refresh_index` unchanged signature. Output order equals `crawl_pptx_paths` order.

- [ ] Add test `refresh_preserves_crawl_order_from_cold`: `refresh_index(fixtures, "L", &[])` paths equal `crawl_pptx_paths(fixtures)`.
- [ ] Implement: collect `(position, path, mtime)` misses. `std::thread::scope` with `available_parallelism()` threads over `chunks`. Each thread returns `Vec<(pos, Option<HymnEntry>)>`. Merge into `Vec<Option<HymnEntry>>` sized like the crawl, fill hits from cache, then `flatten`.
- [ ] Run tests. Commit: `perf(core): parse cache misses in parallel`

### Task 3: Searcher owns matcher, precomputed Utf32 + labels

**Files:**
- Modify: `crates/hymnal-core/src/search.rs`, `model.rs`
- Test: `tests/search_test.rs`

**Interfaces:**
- Produces: `Searcher::search(&mut self, q) -> Vec<SearchHit>`. `Searcher::label(index) -> &str` returns `"150  Title  · Library"` or `"Title  · Library"` when no number. `SearchHit` unchanged.

- [ ] Add test `label_formats_number_title_library` using fixture entry 1 and library "test-lib": expect `"1  Plecaţi-vă…"` prefix and `"  · test-lib"` suffix.
- [ ] Implement: `Indexed { entry, label: String, number: Utf32String, title: Utf32String, filename: Utf32String, body: Utf32String, number_str, title_folded, filename_folded, body_folded }`. Keep folded `String`s for the exact/prefix/contains bonus. Store `matcher: Matcher` on `Searcher`. `search(&mut self)`.
- [ ] Update existing search tests to `let mut s`.
- [ ] Run tests. Commit: `perf(core): searcher reuses matcher and pre-encoded haystacks`

### Task 4: library.rs cleanup, remove `dirs`

**Files:**
- Modify: `crates/hymnal-core/src/library.rs`, `crates/hymnal-core/Cargo.toml`
- Test: inline tests in `library.rs`

**Interfaces:**
- Produces: `Config { hymns_url: String, libraries, download_dir, language, active_theme, output_display }`. `DEFAULT_HYMNS_URL`. `default_library_dir() -> data_dir/hymns`. `hymns_stamp_path() -> data_dir/hymns.etag`. `legacy_clone_dir() -> data_dir/default-library`. `default_library()` path is `default_library_dir()` itself.

- [ ] Test `old_config_with_default_repo_url_still_parses`: TOML with `default_repo_url = "x"` loads and `hymns_url == DEFAULT_HYMNS_URL`.
- [ ] Implement: private `fn dirs() -> Option<ProjectDirs>`. Replace `default_repo_url` with `#[serde(default = "default_hymns_url")] hymns_url`. `downloads_dir` via `directories::UserDirs`. Remove `DEFAULT_REPO_HYMNS_SUBDIR`. Remove `dirs` from Cargo.toml.
- [ ] Update `Config` literals in tests (`hymns_url`).
- [ ] Run tests. Commit: `refactor(core): single dirs helper, hymns_url config, drop dirs crate`

### Task 5: Zip-based sync

**Files:**
- Rewrite: `crates/hymnal-core/src/sync.rs`
- Modify: `crates/hymnal-core/Cargo.toml` (remove `git2`)
- Test: `tests/sync_test.rs` (new, moves the inline test out)

**Interfaces:**
- Produces: `enum SyncOutcome { Downloaded, Unchanged }`. `fn sync_hymns(url: &str, dest: &Path, stamp: &Path) -> anyhow::Result<SyncOutcome>`. `fn extract_zip(zip: &Path, dest: &Path) -> anyhow::Result<()>`.

- [ ] Test harness: `fn serve(body: Vec<u8>, etag: &str) -> (String, JoinHandle)` using `TcpListener` bound to `127.0.0.1:0`. Loop accepting connections. Parse first line. `HEAD` → `200` with `ETag`, `Content-Length`. `GET` → same plus body. Runs until dropped via a `stop` flag or accepts N requests.
- [ ] Tests: `first_sync_downloads_and_extracts` (zip built with `zip::ZipWriter` holding `a/001.pptx`), `second_sync_is_unchanged_when_etag_matches`, `changed_etag_redownloads`, `missing_dest_redownloads_even_with_stamp`.
- [ ] Implement per spec: HEAD, compare stamp, GET to `dest.with_extension("zip.part")`, `extract_zip` into `dest.with_extension("tmp")`, remove old `dest`, rename, write stamp, cleanup part.
- [ ] Remove `git2`. Run tests. Commit: `feat(core): sync hymns from release zip via ETag, drop libgit2`

### Task 6: refresh.rs on the new sync

**Files:**
- Modify: `crates/hymnal-core/src/refresh.rs`
- Test: `tests/refresh_test.rs`

**Interfaces:**
- Produces: `force_clean_paths(hymns_dir, stamp, cache)`. `sync_default(cfg) -> Option<SyncOutcome>`. `default_library_present() -> default_library_dir().is_dir()`. `load_local`, `load_library` unchanged.

- [ ] Update `force_clean` tests for three paths. Add `register_default_library_repairs_legacy_path` via a new pub fn `repair_default_library(cfg: &mut Config)` that sets any `managed_by_git` entry path to `default_library()` path.
- [ ] Implement. In `sync_default`, on `Downloaded` remove `legacy_clone_dir()` if it exists.
- [ ] Run tests. Commit: `refactor(core): refresh pipeline on zip sync, repair legacy library path`

### Task 7: GUI state + module split

**Files:**
- Create: `crates/hymnal-gui/src/{state,i18n,library,downloads,settings,themes,control}.rs`
- Modify: `crates/hymnal-gui/src/main.rs`

**Interfaces:**
- Produces: `state::App` per spec. `state::Msg` enum. Each module `pub fn wire(ui: &AppWindow, app: &App)`. `main.rs` keeps: logging, `App::boot(&ui)`, worker spawn, timer drain, `ui.run()`.

- [ ] Move `apply_language` to `i18n.rs`.
- [ ] Create `state.rs` with `App`, `App::save_cfg`, `App::with_entry`, `Msg`.
- [ ] Move callbacks: library (`query_changed`, `current_changed`, `prev/next_slide`, `open_current`, `reveal_current`, `project_current`), downloads (`choose_folder`, `start_download`, `reveal_download`), settings (`force_sync`, `set_language`, `add/remove/set_library_enabled`, `library_rows`), themes (all `theme_*`, `font_picked`… `set_active`), control (`ctl_*`, `push_to_projector`, `refresh_control_view`).
- [ ] Worker sends `Msg::Index(Searcher)`. Force sync and reindex send `Msg::Synced` / `Msg::Reindexed` with a `Searcher`. Downloader events wrapped `Msg::Download`.
- [ ] Replace every `Config::load(&p)` inside handlers with `app.cfg.borrow()` and `app.save_cfg()`.
- [ ] `cargo build -p hymnal-gui` zero warnings. `cargo clippy` clean.
- [ ] Commit: `refactor(gui): split main.rs by tab, one App state, one message channel`

### Task 8: Release workflow ships hymns.zip

**Files:**
- Modify: `.github/workflows/release.yml`, `README.md`, `CLAUDE.md`

- [ ] Add job `hymns` (ubuntu): checkout, `cd assets/920 && zip -qr ../../hymns.zip .`, upload artifact `hymns`. `release.needs: [build, hymns]`.
- [ ] README: replace git-sync wording with release zip. Config example uses `hymns_url`. CLAUDE.md: update `sync`, `library`, cache notes.
- [ ] Commit: `ci: publish hymns.zip with each release`

### Task 9: Manual QA

- [ ] `cargo run --release -p hymnal-gui` with `RUST_LOG=debug`. Verify: index loads from cache, search, arrows, preview, Enter opens, reveal, Project → Control, start/stop projector, download a short video, force sync (downloads zip or 404s cleanly in dev), language switch, theme save and set active.
- [ ] Record any defect and fix in place.
