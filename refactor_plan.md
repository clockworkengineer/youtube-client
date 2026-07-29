# Refactor Plan: Size & Performance Optimization

This document outlines a concrete plan to optimize the compilation time, binary size, and runtime performance of the YouTube Client workspace.

---

## 1. Current Issues & Findings

After analyzing the workspace crate structure, dependencies, and source files, we identified the following optimization opportunities:

### A. Heavy Transitive Dependencies in GUI Crate
* **Problem**: `youtube-gui` depends on `youtube-client-lib`, which unconditionally compiles and links `rodio` (audio decoding and playback) and `rusty_ytdl` (YouTube video downloading).
* **Impact**:
  * **Compilation Time**: `rodio` pulls in complex audio handling and decoding dependencies (e.g., `cpal`, `symphonia`, `claxon`, `lewton`, `hound`, etc.).
  * **Binary Size**: These unused libraries add megabytes to the `youtube-gui` binary.
  * **Runtime overhead**: Loading unnecessary DLLs and libraries.
* **Solution**: Introduce Cargo feature flags in `youtube-client-lib` to make `audio` (rodio) and `download` (rusty_ytdl) optional. Disable them by default in `youtube-gui`, and enable them explicitly in `youtube-client` (CLI).

### B. Over-allocated Tokio Features
* **Problem**: Both `youtube-client-lib` and `youtube-client` pull in `tokio` with `features = ["full"]`.
* **Impact**: Unnecessary compilation of features like `signal`, `process` (for lib), `net` (redundant with hyper/reqwest handles), and others.
* **Solution**: Trim tokio features to only the required set (e.g., `rt`, `rt-multi-thread`, `macros` for application binaries, and `sync` if needed).

### C. Unoptimized image Crate Features
* **Problem**: `youtube-gui` pulls in the `image` crate with default features or broad format features.
* **Impact**: Compiling encoders/decoders for unused formats (e.g., BMP, TGA, TIFF, WebP, etc.).
* **Solution**: Explicitly disable default features for `image` and only enable `png` and `jpeg` (the formats used for YouTube thumbnails).

---

## 2. Step-by-Step Refactor Plan

### Step 2.1: Implement Feature Flags in `youtube-client-lib`

1. Modify `youtube-client-lib/Cargo.toml` to make `rodio` and `rusty_ytdl` optional:
   ```toml
   [dependencies]
   anyhow.workspace = true
   google-youtube3.workspace = true
   open.workspace = true
   tokio = { workspace = true, features = ["rt", "sync"] } # Trim features
   yup-oauth2.workspace = true
   serde = { workspace = true, features = ["derive"] }
   serde_json.workspace = true

   # Optional dependencies
   rodio = { workspace = true, optional = true }
   rusty_ytdl = { workspace = true, optional = true }

   [features]
   default = ["audio", "download"]
   audio = ["dep:rodio"]
   download = ["dep:rusty_ytdl"]
   ```

2. Wrap optional features in `youtube-client-lib/src/lib.rs` with `#[cfg(feature = "...")]` compilation gates:
   * **`download_video`**: Gate behind `#[cfg(feature = "download")]`.
   * **`play_audio_rodio`**: Gate behind `#[cfg(feature = "audio")]`.

### Step 2.2: Optimize `youtube-client` (CLI) Dependencies

1. Update `youtube-client/Cargo.toml` to:
   * Explicitly enable the `audio` and `download` features of `youtube-client-lib`.
   * Trim `tokio` features to `["rt-multi-thread", "macros"]`.
   * Remove redundant root dependencies if they are only used through the library.

### Step 2.3: Optimize `youtube-gui` (GUI) Dependencies

1. Update `youtube-gui/Cargo.toml` to:
   * Import `youtube-client-lib` with `default-features = false` (completely stripping `rodio`, `cpal`, `rusty_ytdl` and all their sub-dependencies).
   * Restrict `image` to `default-features = false` with only `["png", "jpeg"]` features.
   * Trim `tokio` features to `["rt-multi-thread", "macros"]`.

---

## 3. Anticipated Benefits

| Metric | Before Refactor | After Refactor (Estimate) | Improvement Type |
| :--- | :--- | :--- | :--- |
| **GUI Compile Time** | ~14s (incremental check) | ~7-9s | Developer Experience |
| **GUI Binary Size** | Large (due to `rodio`/`cpal`/`rusty_ytdl`) | Reduced by 15-30% | Deployment/Portability |
| **Dependency Tree Count** | 150+ crates | ~110 crates | Code Maintainability / Auditability |
