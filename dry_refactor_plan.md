# DRY Refactor Plan: Eliminating Code Duplication

This document details duplicate code patterns within the YouTube Client workspace and outlines a plan to eliminate them, improving maintainability, reducing code size, and consolidating shared behaviors.

---

## 1. Identified Code Duplication

### A. Duplicate Rodio Audio Playback Code
* **Location 1**: `youtube-client-lib/src/lib.rs` (in `play_audio_rodio` method)
* **Location 2**: `youtube-client/src/main.rs` (in `Commands::Play` handler)
* **Problem**: The setup of the rodio device sink, mixer connect, audio decoding (`Decoder::new`), and blocking wait is duplicated exactly in both crates.
* **Refactor Plan**: 
  * Remove the duplicate playback code in `youtube-client/src/main.rs`.
  * Define a helper in the library or reuse the existing `play_audio_rodio` method. Since `play_audio_rodio` does not require any `self` client connection state, we will change it to an associated function on `YoutubeClient` or a standalone module function (e.g. `pub fn play_audio_rodio(file_path: &Path)`).
  * Call `youtube_client_lib::play_audio_rodio(&file)` from the CLI.

### B. Duplicate Configuration File Resolution & Parsing
* **Location 1**: `youtube-gui/src/main.rs` (in `get_client_async` and `get_player_path`)
* **Location 2**: `youtube-client/src/main.rs` (in `get_credentials`)
* **Problem**: Both the CLI and GUI manually handle resolving `private_config.json` vs. `config.json`, checking if files exist, reading them, and parsing them into credentials or options using custom deserialization structs.
* **Refactor Plan**:
  * Consolidate configuration management in `youtube-client-lib`.
  * Define a common `Config` struct in the library that knows how to load/resolve itself from standard paths (`private_config.json` or `config.json`).
  * Expose this struct to both GUI and CLI to keep configuration loading unified.

### C. Duplicate Thumbnail Fetching Logic in GUI
* **Location 1**: `youtube-gui/src/main.rs` (in `View::Subscriptions` rendering)
* **Location 2**: `youtube-gui/src/main.rs` (in `View::ChannelVideos` rendering)
* **Problem**: Staging the thumbnail fetch in the state cache, launching the async tokio task to fetch/convert image bytes, and returning the cloned texture is duplicated.
* **Refactor Plan**:
  * Extract this to a helper method in `youtube-gui`:
    ```rust
    fn get_or_fetch_thumbnail(&self, ctx: &egui::Context, id: &str, url: &str) -> Option<egui::TextureHandle>
    ```
  * Replace the duplicated blocks in the UI views with a call to this method.

---

## 2. Step-by-Step Refactoring Tasks

### Step 1: Consolidate Configuration & Audio in `youtube-client-lib`
1. Add `Config` structure and file resolution function in `youtube-client-lib/src/lib.rs` (or a `config` module).
2. Change `play_audio_rodio` in `youtube-client-lib/src/lib.rs` to a standalone public function or associated function that does not require `&self`.

### Step 2: Clean up CLI (`youtube-client`)
1. Replace local file configuration parsing in `youtube-client/src/main.rs` with the library's `Config` utility.
2. Replace local rodio player implementation with `youtube_client_lib::play_audio_rodio(...)`.

### Step 3: Clean up GUI (`youtube-gui`)
1. Replace local file configuration parsing and `get_player_path` with the library's consolidated `Config`.
2. Extract the duplicate thumbnail loading block into `get_or_fetch_thumbnail`.
