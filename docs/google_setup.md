# Google OAuth2 Credentials Setup Guide

This guide walks you through setting up Google Cloud Console credentials with the necessary API scopes to authenticate the YouTube Client application suite (`youtube-client` and `youtube-gui`).

---

## Step 1: Create a Google Cloud Project

1. Open the [Google Cloud Console](https://console.cloud.google.com/).
2. Log in with your personal Google account.
3. Click the project dropdown at the top navigation bar and select **New Project**.
4. Give your project a name (e.g. `YouTube Client`) and click **Create**.

---

## Step 2: Enable the YouTube Data API v3

1. In the console's top search bar, search for **YouTube Data API v3**.
2. Select **YouTube Data API v3** from the Marketplace/API library list.
3. Click the blue **Enable** button.

---

## Step 3: Configure the OAuth Consent Screen & Scopes

If this is a new project, Google requires setting up the consent screen:

1. In the left navigation menu, go to **APIs & Services** > **OAuth consent screen**.
2. Select user type **External** (accessible to your personal Google account) and click **Create**.
3. Fill in the required fields:
   * **App name**: `YouTube Client Desktop`
   * **User support email**: Select your email address.
   * **Developer contact information**: Enter your email address.
4. Click **Save and Continue**.

### ⚠️ Critical Step: Configure Required Scopes

To enable interactive features (rating videos, creating playlists, posting comments, and managing subscriptions), you must configure **all three** of the following scopes:

1. Click **Add or Remove Scopes**.
2. In the filter box, search for `youtube` and check the following scopes:

| Scope URI | Scope Name | Why It Is Required |
| :--- | :--- | :--- |
| `https://www.googleapis.com/auth/youtube` | Manage your YouTube account | Required for subscribing, unsubscribing, and rating videos. |
| `https://www.googleapis.com/auth/youtube.force-ssl` | See, edit, and permanently delete YouTube content | Required for creating/deleting playlists and posting comments. |
| `https://www.googleapis.com/auth/youtube.readonly` | View your YouTube account | Required for fetching subscriptions, uploads, and feeds. |

3. Click **Update** at the bottom of the drawer, then click **Save and Continue**.

### Test Users Configuration

Because the OAuth app is in "Testing" mode, Google will only allow specified test users to authenticate:
1. Under **Test users**, click **+ Add Users**.
2. Enter the Google email address of the account you plan to use with the client.
3. Click **Add**, then click **Save and Continue**.

---

## Step 4: Create OAuth 2.0 Desktop Credentials

1. In the left sidebar, navigate to **APIs & Services** > **Credentials**.
2. Click **+ Create Credentials** at the top and select **OAuth client ID**.
3. Under **Application type**, choose **Desktop app**.
4. Enter a name (e.g. `YouTube Client Desktop App`).
5. Click **Create**.
6. A dialog box will display your **Client ID** and **Client Secret**. Copy these values.

---

## Step 5: Configure Credentials in the Application

You can provide your credentials using any of the following methods (see [Configuration Guide](configuration.md) for full details):

### Method A: `private_config.json` (Recommended)
Create a file named `private_config.json` in the root directory (this file is git-ignored for safety):
```json
{
  "client_id": "YOUR_CLIENT_ID.apps.googleusercontent.com",
  "client_secret": "YOUR_CLIENT_SECRET"
}
```

### Method B: Environment Variables
Export variables in your terminal:
```bash
# Linux / macOS
export GOOGLE_CLIENT_ID="YOUR_CLIENT_ID.apps.googleusercontent.com"
export GOOGLE_CLIENT_SECRET="YOUR_CLIENT_SECRET"

# Windows PowerShell
$env:GOOGLE_CLIENT_ID="YOUR_CLIENT_ID.apps.googleusercontent.com"
$env:GOOGLE_CLIENT_SECRET="YOUR_CLIENT_SECRET"
```

### Method C: Global Configuration
Place `config.json` in your operating system's application directory (`%APPDATA%\youtube-client\config.json` on Windows or `~/.config/youtube-client/config.json` on Linux).

---

## Step 6: Authenticate

Once configured, run the interactive login command:
```bash
cargo run --bin youtube-client -- login
```
A browser window will open. Sign in with the test user account you registered in Step 3 and grant the requested permissions. Your refresh token will be cached to `tokencache.json`.

---

## Upgrading Scopes & Resetting Cached Tokens

If you previously authenticated with read-only scopes and are encountering `403 Forbidden` errors when rating, commenting, or modifying playlists:
1. Delete the outdated token cache file:
   ```bash
   rm tokencache.json
   ```
2. Re-run `cargo run --bin youtube-client -- login` to authorize the new read/write scopes in your browser.
