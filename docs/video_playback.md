# Setting Up Video Playback

This guide explains how the YouTube Client application handles video playback when you click a video, and how to configure your preferred media player for seamless streaming.

---

## How Playback Works

When you click on a video in the GUI application, the client attempts to stream the video directly using a native media player on your system. It searches for media players in the following order:

1. **Configured Media Player**: The path specified in your configuration (`player_path`).
2. **System Path Players**: Looks for `mpv` or `vlc` in your system's `PATH`.
3. **Common VLC Installation Paths**:
   - `C:\Program Files\VideoLAN\VLC\vlc.exe`
   - `C:\Program Files (x86)\VideoLAN\VLC\vlc.exe`
4. **Browser Fallback**: If no media player is found or starts successfully, the video will open in your system's default web browser.

---

## Step 1: Install a Media Player (Recommended)

For the best experience, we recommend installing either **MPV** or **VLC**.

### Option A: MPV (Recommended for lightweight streaming)
1. Download and install **mpv** from [mpv.io](https://mpv.io/).
2. **Crucial**: MPV relies on `yt-dlp` (or `youtube-dl`) to stream YouTube videos.
   - Download the latest version of `yt-dlp` from [yt-dlp GitHub releases](https://github.com/yt-dlp/yt-dlp).
   - Place the `yt-dlp.exe` executable in the same folder as `mpv.exe`, or add it to your system's environment `PATH` variables.

### Option B: VLC Media Player
1. Download and install VLC from [VideoLAN](https://www.videolan.org/).
2. Make sure the VLC YouTube script is up-to-date. If VLC fails to play YouTube videos:
   - Download the latest `youtube.luac` from the official [VLC Git Repository](https://code.videolan.org/videolan/vlc/-/raw/master/share/lua/playlist/youtube.lua).
   - Place it in your VLC playlist directory (typically `C:\Program Files\VideoLAN\VLC\lua\playlist\`, replacing the existing one).

---

## Step 2: Configure Your Player Path

If your media player is not installed in the default location or you want to use a specific player, you can specify its executable path in your configuration file (`private_config.json` or `config.json`).

### Example Configuration (`private_config.json`)

Add the `player_path` property pointing to your media player executable:

```json
{
  "client_id": "YOUR_CLIENT_ID",
  "client_secret": "YOUR_CLIENT_SECRET",
  "player_path": "C:\\Program Files\\mpv\\mpv.exe"
}
```

> [!NOTE]
> - Always use double backslashes (`\\`) in JSON file paths on Windows.
> - `private_config.json` is ignored by git, making it the safest place to store your local settings.

---

## Troubleshooting

### Video Opens in Web Browser Instead
* Check that your `player_path` in `private_config.json` is correct and uses double backslashes.
* If relying on the system `PATH`, make sure you can run the player (e.g., `mpv` or `vlc`) from a terminal.

### Player Opens but Fails to Load/Play the Video (HTTP 403 Forbidden)
* **For MPV**:
  * Ensure `yt-dlp.exe` is in your system `PATH` or the MPV directory, and that it is up-to-date (`yt-dlp -U`).
  * If YouTube returns `HTTP error 403 Forbidden`, YouTube may be blocking the default `ANDROID_VR` client format requested by `yt-dlp`. The application automatically passes `--ytdl-raw-options=extractor-args=youtube:player_client=mweb` when launching MPV. If running MPV manually from command line, run:
    ```bash
    mpv "https://www.youtube.com/watch?v=..." --ytdl-raw-options=extractor-args=youtube:player_client=mweb
    ```
* **For VLC**: Update your `youtube.luac` file as described in Step 1.
