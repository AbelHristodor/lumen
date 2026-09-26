# Refactor: clean core, split GUI, zip-based hymn sync

Date: 2026-09-26

## Goal

Behavior-preserving refactor of the whole workspace. Every user-visible feature
stays. Three outcomes:

1. Clean code. `main.rs` split by feature. One shared app state. No duplicate
   config loads. No dead code.
2. Performance. Faster cold index, faster per-keystroke search, smaller cache,
   no libgit2 in the binary or the build.
3. Hymn distribution via a `hymns.zip` release asset instead of cloning the
   whole app repository.

## Non-goals

- No feature removal. Themes, i18n, self-update, multi-library, projector stay.
- No UI redesign. `app.slint` layout is untouched except where a property name
  changes.
- No new dependencies. `git2` and `dirs` are removed. Nothing is added.

## Core crate

### model.rs

`HymnEntry` drops `body`. `slides` is the single source. Search text is derived
at `Searcher::new`. `CACHE_VERSION` bumps to 7 so old caches re-parse.

### pptx.rs

`ParsedPptx` drops `body`. No other change. Tests assert on `slides.join`.

### index.rs

- `build_index` deleted. `refresh_index(root, lib, &[])` is the same thing.
- `refresh_index` uses a `HashMap<&Path, &HymnEntry>` for cache lookup.
- Parsing of cache misses runs in parallel with `std::thread::scope`, one thread
  per `available_parallelism`, static chunking. Output order stays the crawl
  order.
- `eprintln!` becomes `log::warn!`.

### search.rs

- `Indexed` stores `Utf32String` per field, folded once at construction, plus a
  precomputed row `label` so the GUI does no per-keystroke formatting.
- `Searcher` owns its `Matcher`. `search` takes `&mut self`.
- Ranking formula unchanged.
- Body text is `fold(slides.join("\n"))`.

### library.rs

- One private `dirs()` returning `ProjectDirs`. All path helpers use it.
- `downloads_dir` uses `directories::UserDirs`. `dirs` crate removed.
- `Config.default_repo_url` is replaced by `hymns_url` with a serde default of
  the release asset URL. Old configs still parse. Unknown fields are ignored.
- Default library path becomes `data_dir/hymns`. `DEFAULT_REPO_HYMNS_SUBDIR`
  removed.
- Multi-library add/remove/toggle functions stay.

### sync.rs

Rewritten. No git.

```
pub enum SyncOutcome { Downloaded, Unchanged }
pub fn sync_hymns(url: &str, dest: &Path, stamp: &Path) -> Result<SyncOutcome>
```

1. `HEAD url` following redirects. Read `ETag`.
2. If `stamp` holds the same ETag and `dest` exists, return `Unchanged`.
3. Else `GET url` to `dest.tmp.zip`, extract into `dest.tmp/`, swap into
   `dest`, write ETag to `stamp`, return `Downloaded`.
4. Extraction rejects entries whose normalized path escapes `dest`.

Tested with a `std::net::TcpListener` HTTP stub that serves HEAD and GET.

### refresh.rs

- `force_clean_paths(hymns_dir, stamp, cache)`.
- `register_default_library` also repairs a stale managed entry whose path is
  the old git clone location. The old `default-library` directory is deleted
  when found.
- `sync_default` calls `sync_hymns`.
- `load_local`, `load_library`, `default_library_present` keep their contracts.

### Removed deps

`git2`, `dirs`. `tar` and `xz2` stay for the Linux ffmpeg archive.

## GUI crate

### Module split

```
src/
  main.rs        bootstrap, logging, worker thread, timer poll, run
  state.rs       App: shared Rc<RefCell<..>> handles + config persistence
  i18n.rs        apply_language
  library.rs     search, preview, open, reveal, project button
  downloads.rs   choose folder, start, reveal, event drain
  settings.rs    force sync, language, library folders
  themes.rs      editor + store wiring
  control.rs     projection + control search
  projector.rs   unchanged
  fonts.rs       unchanged
```

Each module exposes `pub fn wire(ui: &AppWindow, app: &App)`.

### App state

```rust
#[derive(Clone)]
pub struct App {
    pub cfg: Rc<RefCell<Config>>,
    pub cfg_path: Option<PathBuf>,
    pub strings: Rc<RefCell<Strings>>,
    pub searcher: Rc<RefCell<Option<Searcher>>>,
    pub rows: Rc<RefCell<Vec<usize>>>,       // library tab row -> entry
    pub ctl_rows: Rc<RefCell<Vec<usize>>>,   // control tab row -> entry
    pub present: Rc<RefCell<PresentationState>>,
    pub projector: Rc<RefCell<Option<ProjectorWindow>>>,
    pub active_theme: Rc<RefCell<Theme>>,
    pub draft: Rc<RefCell<Theme>>,
    pub themes: Rc<RefCell<Vec<Theme>>>,
    pub families: Rc<Vec<String>>,
    pub displays: Rc<Vec<Display>>,
}
impl App {
    pub fn save_cfg(&self)   // one place that writes config.toml
    pub fn with_entry<R>(&self, rows: &RefCell<Vec<usize>>, row: i32, f) -> Option<R>
}
```

Config is loaded once. Every handler that today re-reads `config.toml` uses
`app.cfg` and `app.save_cfg()`.

### Worker results

Worker threads build the `Searcher` and send it over the channel. The UI
thread only swaps the handle. One `Msg` enum replaces four channels:

```rust
enum Msg {
    Index(Searcher),
    Synced(Result<Searcher, String>),
    Reindexed(Result<Searcher, String>),
    Download(DownloadEvent),
}
```

One 200 ms `Timer` drains it.

## Release workflow

New job `hymns` on ubuntu: `zip -r hymns.zip .` inside `assets/920`, upload
as artifact. The `release` job already publishes `artifacts/*`, so the zip
lands beside the binaries.

Consequence: hymn updates ship with a release. A plain git push to `main`
no longer reaches users.

## Migration for existing installs

- Old cache: version mismatch, re-parsed once.
- Old config: `default_repo_url` ignored, `hymns_url` defaults. Managed
  library path repaired to `data_dir/hymns`.
- Old clone at `data_dir/default-library`: deleted on first successful sync.

## Testing

All existing tests keep passing after mechanical updates. New tests:

- `sync_test`: stub server, first call `Downloaded`, second `Unchanged`,
  changed ETag `Downloaded` again, zip-slip entry rejected.
- `index_test`: parallel refresh yields the same entries as serial, in crawl
  order.
- `search_test`: `label` matches previous `row_label` format.

Manual QA: run the app, search, arrow keys, preview, project, download,
force sync, language switch, theme save.
