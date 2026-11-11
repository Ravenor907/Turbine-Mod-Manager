# Turbine browser helper

A small browser extension that sends mods you download from Nexus Mods straight into Turbine, and shows what Turbine is doing right on the Nexus page.

Works in Chrome, Brave, Edge, Chromium and Vivaldi (version 116 or newer).

## What it does

- **Downloads:** when a download comes from Nexus (a "Slow download" or any file from nexusmods.com), it tells Turbine where the file landed so Turbine can add it to the right game. The file still goes to your normal Downloads folder.
- **Mod manager download links:** while Turbine is running, a click on a Nexus "Mod manager download" link goes to Turbine directly. When Turbine is not running, the browser handles the link the usual way.
- **Panel on Nexus pages:** a small Turbine panel in the bottom-right corner shows progress, "Added to (game)" with notes and any other mods it needs, and the installer's options when a mod asks you to choose. Tap the panel to open it and "Hide" to shrink it back. Drag it by its title bar to move it.
- **Toolbar button:** shows whether Turbine is connected and what it is doing. The badge shows progress, OK, or ! when something needs a look.
- **Opens the mod's page in Game Mode:** Game Mode only shows apps Steam starts, so Turbine starts your browser through Steam (add it once as a non-Steam game) and asks the helper to open the Nexus page in a new tab. It only ever opens nexusmods.com pages this way.
- **Keeps itself up to date:** when Turbine has a newer helper, it copies the new files into the folder in Downloads you loaded the helper from and asks the helper to reload from there. Helpers from Turbine 0.53 and earlier can't do this yet: once, open chrome://extensions and press the reload arrow on Turbine browser helper.
- **Game hint:** it tells Turbine which mod page you are on, so Turbine knows the game. It reads only the page address.
- **Sign in with my browser (Google and others):** Turbine has its own Nexus page, and Google doesn't allow signing in inside an app's own browser. When you press "Sign in with my browser" in Turbine, the helper opens Turbine's sign-in page in Chrome: "Turbine is waiting for you to sign in to Nexus Mods". Press "Sign in to Nexus Mods" there and sign in (with Google or anything else); a banner across the top of Nexus's pages says Turbine is waiting. As soon as Chrome is signed in to Nexus (already signed in counts), the helper sends Nexus's sign-in cookies (nexusmods.com only, never any other site's) to Turbine on this computer, Turbine signs its page in with them, and the page says "Signed in. Go back to Turbine" (in Game Mode Turbine comes back by itself; otherwise press the Steam button and pick Turbine). Nothing is sent unless you pressed the button in Turbine in the last 15 minutes. This is why the helper asks for the cookies permission.
- **Your API key, only if you choose:** after signing in, the page offers "Give Turbine my API key too". If you press it, the tab opens Nexus's API Keys page, the helper copies the Personal API Key from it (only that one, never another app's) and sends it to Turbine on this computer, which checks it with Nexus and keeps it like a pasted key. The helper never stores it.

It never clicks or changes anything on Nexus pages (opening a tab is all it does), and it only talks to Turbine on this computer (127.0.0.1).

## Load it

1. Open the browser's extensions page (`chrome://extensions`, `brave://extensions`, `edge://extensions` or `vivaldi://extensions`).
2. Turn on **Developer mode**.
3. Choose **Load unpacked** and pick this folder.

## Pair it with Turbine

In Turbine, open Settings, then Accounts, and choose **Pair again**. Turbine opens a page in your browser; the extension reads it and shows "Paired with Turbine". You only do this once, or again if Turbine asks.

## Tests

With Node 22 or newer (nothing to install), as the build runs them:

```
cd extension
node test/test.js
```

`test/handshake.js` also needs Turbine's stand-in (`tools/signin-standin.rs`, built by the build) in `TURBINE_STANDIN`.
