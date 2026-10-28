# SOLID Clean Architecture Specification

**Workspace:** `youtube-client`  
**Target Version:** 0.2.0  
**Status:** Implemented & Verified  

---

## 1. Overview & Architectural Goals

The `youtube-client` workspace was refactored under the **SOLID** design principles to convert monolithic components and tightly-coupled dependencies into an extensible, testable, and maintainable Clean Architecture:

* **S — Single Responsibility Principle (SRP):** Each struct, module, and handler has one well-defined reason to change.
* **O — Open/Closed Principle (OCP):** New media players, subscription formats, and output renderers can be added via traits without modifying core dispatchers.
* **L — Liskov Substitution Principle (LSP):** In-memory test doubles (`MockYoutubeClient`) and live clients (`YoutubeClient`) satisfy identical behavioral contracts via `YoutubeBackend`.
* **I — Interface Segregation Principle (ISP):** Loose, bloated function parameters are grouped into domain-specific contexts (`VideoDetailsContext`).
* **D — Dependency Inversion Principle (DIP):** Consumer crates (`youtube-gui` and `youtube-client`) depend on high-level service abstractions, not concrete Google API client structs.

---

## 2. Phase 1: Dependency Inversion & Pluggable Backend (DIP & LSP)

### Problem Addressed
Previously, `youtube-gui` and `youtube-client` directly constructed and held references to `Arc<YoutubeClient>`. This prevented offline unit and integration testing and tightly coupled UI logic to Google network requests.

### Solution Architecture
1. **Composite Service Trait:**
   Defined [`YoutubeApiService`](file:///c:/Projects/youtube-client/youtube-client-lib/src/traits/mod.rs) combining segregated service traits:
   ```rust
   pub trait YoutubeApiService:
       VideoService + SubscriptionService + PlaylistService + CommentService + Send + Sync
   {}

   impl<T> YoutubeApiService for T where
       T: VideoService + SubscriptionService + PlaylistService + CommentService + Send + Sync
   {}
   ```
2. **Unified `YoutubeBackend` Adapter:**
   Introduced an enum adapter avoiding runtime trait boxing:
   ```rust
   pub enum YoutubeBackend {
       Live(Arc<YoutubeClient>),
       Mock(MockYoutubeClient),
   }
   ```
   Both variants implement `VideoService`, `SubscriptionService`, `PlaylistService`, `CommentService`, `MediaDownloader`, and pagination methods identically.
3. **Pluggable Client Context & State:**
   - CLI: [`CliContext`](file:///c:/Projects/youtube-client/youtube-client/src/commands/context.rs) provides `CliContext::with_mock(mock)`.
   - GUI: [`AppState`](file:///c:/Projects/youtube-client/youtube-gui/src/types.rs) supports `AppState::with_mock_backend(mock)`.

---

## 3. Phase 2: Pluggable Media Players & Registry (OCP)

### Problem Addressed
Media player launching was previously hardcoded via string matching on `"mpv"` and `"vlc"`, making it impossible to add new players (such as IINA, Celluloid, or custom wrapper scripts) without modifying library code.

### Solution Architecture
1. **`MediaPlayer` Trait:**
   ```rust
   pub struct PlayOptions {
       pub start_secs: Option<f32>,
       pub cookies_file: Option<PathBuf>,
       pub user_agent: Option<String>,
       pub log_file: Option<PathBuf>,
   }

   pub trait MediaPlayer: Send + Sync {
       fn id(&self) -> &'static str;
       fn display_name(&self) -> &'static str;
       fn is_available(&self) -> bool;
       fn launch_stream(&self, url: &str, title: &str, opts: &PlayOptions) -> Result<std::process::Child, String>;
       fn launch_file(&self, path: &Path, title: &str, opts: &PlayOptions) -> Result<std::process::Child, String>;
   }
   ```
2. **Concrete Implementations:**
   - [`MpvPlayer`](file:///c:/Projects/youtube-client/youtube-client-lib/src/player.rs): Encapsulates `--start`, `--title`, `--ytdl-raw-options`.
   - [`VlcPlayer`](file:///c:/Projects/youtube-client/youtube-client-lib/src/player.rs): Encapsulates `--start-time`, `--meta-title`.
   - [`IinaPlayer`](file:///c:/Projects/youtube-client/youtube-client-lib/src/player.rs): Encapsulates macOS IINA flags.
   - [`CustomExecutablePlayer`](file:///c:/Projects/youtube-client/youtube-client-lib/src/player.rs): Launches arbitrary user-specified binary paths.
   - [`SystemDefaultPlayer`](file:///c:/Projects/youtube-client/youtube-client-lib/src/player.rs): Launches via `open::that`.
3. **`PlayerRegistry`:**
   Auto-discovers available players on the system and prioritizes selection according to user configuration.

---

## 4. Phase 3: Pluggable Subscription Importers (Strategy Pattern)

### Problem Addressed
Subscription import and export previously hardcoded ad-hoc file inspection for OPML vs CSV in GUI and library code.

### Solution Architecture
1. **`SubscriptionFormat` Trait:**
   ```rust
   pub trait SubscriptionFormat: Send + Sync {
       fn id(&self) -> &'static str;
       fn display_name(&self) -> &'static str;
       fn default_extension(&self) -> &'static str;
       fn can_parse(&self, content: &str, path: Option<&Path>) -> bool;
       fn parse(&self, content: &str) -> Result<Vec<SubscriptionImport>, String>;
       fn serialize(&self, subs: &[SubscriptionImport]) -> Result<String, String>;
   }
   ```
2. **Standard Strategies:**
   - [`OpmlFormat`](file:///c:/Projects/youtube-client/youtube-client-lib/src/importers.rs): Parses and outputs standard OPML XML (FreeTube, NewPipe, Feedly).
   - [`TakeoutCsvFormat`](file:///c:/Projects/youtube-client/youtube-client-lib/src/importers.rs): Parses Google Takeout `subscriptions.csv`.
   - [`NewPipeJsonFormat`](file:///c:/Projects/youtube-client/youtube-client-lib/src/importers.rs): Parses NewPipe backup JSON format.
3. **`SubscriptionFormatRegistry`:**
   Iterates through registered strategies and auto-detects format via `can_parse(...)` without user intervention.

---

## 5. Phase 4: GUI Domain Handlers & Interface Segregation (SRP & ISP)

### Problem Addressed
In `youtube-gui`, `main.rs` contained a single 420-line monolithic `match pending_action` block that handled authentication, downloads, audio playback, file system queries, and window states. Furthermore, `render_details_view` accepted 12 individual function parameters.

### Solution Architecture
1. **Domain Handlers:**
   Decomposed action dispatching into domain modules under [`youtube-gui/src/handlers/`](file:///c:/Projects/youtube-client/youtube-gui/src/handlers/):
   - `auth.rs`: Handles OAuth login, token saving, and account disconnection.
   - `navigation.rs`: Handles view history stack and routing.
   - `playback.rs`: Handles local Rodio audio dispatch, external player execution with start offsets, and browser URLs.
   - `download.rs`: Handles `yt-dlp` download task spawning and progress updates.
   - `settings.rs`: Handles configuration persistence and downloads directory syncing.
   - `library.rs`: Handles channel loading, subscriptions, playlists, details, comments, and OPML/CSV imports.
2. **`VideoDetailsContext`:**
   Consolidated the 12 loose parameters into a single cohesive structure:
   ```rust
   pub struct VideoDetailsContext<'a> {
       pub video: &'a Video,
       pub details: Option<&'a VideoDetails>,
       pub comments: Option<&'a Vec<Comment>>,
       pub rating: Option<Rating>,
       pub comment_input: &'a mut String,
       pub download_status: Option<DownloadStatus>,
       pub play_options: &'a PlayOptions,
   }
   ```

---

## 6. Phase 5: CLI Output Formatting Decoupling (SRP & OCP)

### Problem Addressed
CLI subcommands previously intermixed network queries, error handling, column width calculations, and stdout printing.

### Solution Architecture
1. **`OutputFormatter<T>` Trait:**
   ```rust
   pub trait OutputFormatter<T> {
       fn format_item(&self, item: &T) -> String;
       fn format_list(&self, items: &[T]) -> String;
       fn format_page(&self, page: &Page<T>) -> String;
   }
   ```
2. **Implementations:**
   - `JsonFormatter`: Pretty-printed JSON serialization for all `serde::Serialize` models.
   - `CsvFormatter`: RFC-4180 compliant CSV generator with quotation escaping for `Video`, `Subscription`, `Playlist`, and `Comment`.
   - `TableFormatter`: Aligned ASCII tables and summary cards.
3. **Pure String Formatter:**
   Added [`format_table`](file:///c:/Projects/youtube-client/youtube-client-lib/src/utils.rs), separating table layout from terminal `stdout` side effects.

---

## 7. Developer Extension Recipes

### Recipe A: Adding a New Media Player
1. Create a struct implementing `MediaPlayer` in `youtube-client-lib/src/player.rs`:
   ```rust
   pub struct CelluloidPlayer;
   impl MediaPlayer for CelluloidPlayer {
       fn id(&self) -> &'static str { "celluloid" }
       fn display_name(&self) -> &'static str { "Celluloid" }
       fn is_available(&self) -> bool { which_in_path("celluloid") }
       fn launch_stream(&self, url: &str, title: &str, opts: &PlayOptions) -> Result<Child, String> {
           // Command execution
       }
       fn launch_file(&self, path: &Path, title: &str, opts: &PlayOptions) -> Result<Child, String> {
           // Command execution
       }
   }
   ```
2. Register the instance in `PlayerRegistry::new()`.

### Recipe B: Adding a New Subscription Import Format
1. Create a struct implementing `SubscriptionFormat` in `youtube-client-lib/src/importers.rs`:
   ```rust
   pub struct MyCustomFormat;
   impl SubscriptionFormat for MyCustomFormat {
       fn id(&self) -> &'static str { "custom" }
       fn display_name(&self) -> &'static str { "Custom Backup" }
       fn default_extension(&self) -> &'static str { "custom" }
       fn can_parse(&self, content: &str, path: Option<&Path>) -> bool { ... }
       fn parse(&self, content: &str) -> Result<Vec<SubscriptionImport>, String> { ... }
       fn serialize(&self, subs: &[SubscriptionImport]) -> Result<String, String> { ... }
   }
   ```
2. Register the strategy in `SubscriptionFormatRegistry::new()`.
