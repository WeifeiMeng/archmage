# Install Archmage on macOS

Archmage 1.0.0 is a Preview for **Apple Silicon** Macs running **macOS 14 or later**. It is not compatible with Intel Macs. The current package is **unsigned** and not notarized because the project does not yet have an Apple Developer identity; there is no automatic updater.

## 1. Download and verify

Open [GitHub Releases](https://github.com/WeifeiMeng/archmage/releases) and download both:

- `Archmage_1.0.0_aarch64.dmg`
- `Archmage_1.0.0_aarch64.dmg.sha256`

In Terminal, change to the download directory and verify the DMG before opening it:

```bash
shasum -a 256 -c Archmage_1.0.0_aarch64.dmg.sha256
```

Continue only when the result says `OK`. Delete both files and download them again if verification fails.

## 2. Install and open for the first time

1. Open `Archmage_1.0.0_aarch64.dmg`.
2. Drag `Archmage.app` into Applications.
3. In Finder, open Applications, right-click Archmage, choose **Open**, and confirm **Open**.
4. If macOS still blocks the app, open System Settings → Privacy & Security, find the Archmage message, and choose **Open Anyway**.

These extra steps are expected for this unsigned Preview. Removing the quarantine attribute with `xattr` is not the normal installation path: it removes an operating-system safety signal and makes it easier to open an unverified download. Verify SHA-256 and use Finder or Privacy & Security instead.

## 3. Complete first launch and configure a Provider

Archmage does not bundle Codex or another Agent Provider. Provider setup remains a separate, user-controlled process:

1. install the Provider using its own trusted distribution channel;
2. authenticate with the Provider outside Archmage when required;
3. open Agent management in Archmage and discover or add the Provider;
4. run verification and confirm that the UI reports it as verified before assigning it to a Capability.

Installed, authenticated, and verified are different states. A missing or unauthenticated Provider should show setup guidance; it is not repaired by reinstalling the Archmage DMG.

## 4. Manual upgrade and data preservation

Archmage currently supports **manual upgrade** only:

1. stop active Runs and quit Archmage;
2. back up `~/Library/Application Support/dev.archmage.archmage`;
3. download and verify the new DMG;
4. replace `Archmage.app` in Applications.

Replacing the app preserves the Application Support directory, including Runs, Teams, Capability configuration, Agent records, artifacts, and managed workspaces. Do not move the data directory into the `.app` bundle.

Before a downgrade, keep a separate backup. A newer release may migrate the database or artifacts to a schema an older release cannot read. The project does not guarantee in-place downgrade compatibility; restoring the matching backup is the safe rollback boundary.

## 5. Uninstall

For an **app-only uninstall**, quit Archmage and move `/Applications/Archmage.app` to the Trash. This preserves Application Support data for a later reinstall.

Deleting `~/Library/Application Support/dev.archmage.archmage` is a separate, irreversible data removal. It deletes Archmage's database, Runs, artifacts, configuration, and managed workspaces. Back it up first, confirm no Run or Agent process is active, and delete it only when you explicitly want to erase all local Archmage data. Provider applications and their credentials are managed separately.

## 6. Troubleshooting

- **Wrong architecture:** run `uname -m`; the supported result is `arm64`. This build does not run on `x86_64` Intel Macs.
- **Unsupported system:** use About This Mac to confirm macOS 14 or later.
- **Gatekeeper blocks launch:** verify SHA-256 again, then use Finder's right-click **Open** or Privacy & Security → **Open Anyway**.
- **“Damaged” DMG or failed checksum:** delete it and both checksum files, then download again from GitHub Releases. Do not disable Gatekeeper.
- **Provider missing or not verified:** install and authenticate that Provider separately, then return to Agent management and run verification.
- **App opens but prior data is absent:** quit the app and check whether `ARCHMAGE_HOME` was set to a different directory before launch.
