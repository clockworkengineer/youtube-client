# Google OAuth2 Credentials Setup Guide

This guide helps you set up the Google Cloud Console credentials needed to authenticate this YouTube Client application.

---

## Step 1: Create a Google Cloud Project

1. Open the [Google Cloud Console](https://console.cloud.google.com/).
2. Log in with your Google account.
3. Click the project dropdown at the top navigation bar and select **New Project**.
4. Give your project a name (e.g. `Youtube CLI Client`) and click **Create**.

---

## Step 2: Enable the YouTube Data API v3

1. In the console's search bar, search for **Library** and select **API Library**.
2. Search for **YouTube Data API v3**.
3. Select it from the list and click the **Enable** button.

---

## Step 3: Configure the OAuth Consent Screen

If this is a new project, Google requires configuring the consent screen:

1. Navigate to **APIs & Services** > **OAuth consent screen** using the left sidebar.
2. Select **External** (available to any Google Account) and click **Create**.
3. Fill in the required fields:
   - **App name**: (e.g. `Youtube CLI Client`)
   - **User support email**: Select your email address.
   - **Developer contact information**: Enter your email address.
4. Click **Save and Continue**.
5. **Scopes**:
   - Click **Add or Remove Scopes**.
   - Search for `/auth/youtube.readonly` and check it.
   - Click **Update** at the bottom, then click **Save and Continue**.
6. **Test Users** (CRITICAL):
   - Click **Add Users**.
   - Enter your personal Google email address (the one you want to sign in with).
   - Click **Add** -> **Save and Continue**.

---

## Step 4: Create OAuth 2.0 Credentials

1. Navigate to **APIs & Services** > **Credentials**.
2. Click **Create Credentials** at the top and select **OAuth client ID**.
3. Under **Application type**, choose **Desktop app**.
4. Enter a name (e.g. `Youtube CLI Desktop Client`).
5. Click **Create**.
6. A dialog box will display your **Client ID** and **Client Secret**. Copy these values.

---

## Step 5: Configure the Application

Create a `config.json` file in the root of the project directory (or modify the template):

```json
{
  "client_id": "YOUR_CLIENT_ID_HERE",
  "client_secret": "YOUR_CLIENT_SECRET_HERE"
}
```

Now you are ready to log in! Run the login command:
```powershell
cargo run --bin youtube-client -- login
```
