# teurtask Desktop App (Tauri)

Native desktop wrapper for the teurtask web frontend. Builds for Linux, Windows, and macOS.

## Local development

### Prerequisites

**All platforms:** Rust toolchain — https://rustup.rs

**Linux (Arch/CachyOS):**
```
sudo pacman -S webkit2gtk-4.1 base-devel openssl libayatana-appindicator
```

**Linux (Debian/Ubuntu):**
```
sudo apt-get install libwebkit2gtk-4.1-dev build-essential libssl-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev
```

**macOS:** Xcode Command Line Tools — `xcode-select --install`

**Windows:** Microsoft C++ Build Tools + WebView2 (usually pre-installed on Win10+)

### Run in dev mode (hot-reload)
```
npm run tauri:dev
```

### Build installer
```
npm run tauri:build
```
Outputs land in `src-tauri/target/release/bundle/`.

### Measure resource usage (Linux)
```
src-tauri/scripts/resource-monitor.sh 60 2 /tmp/teurtask.csv   # seconds, interval, optional CSV
```
Samples CPU (per interval) and RSS of the app and its WebKit children (`web`, `network`) and
prints avg/max per process. Measure with the window unfocused to see the idle cost.

### Linux media notes
- Mic/camera: WebKitGTK denies `getUserMedia` unless the app grants it; `enable_media_capture`
  in `src/lib.rs` does it for the teurtask origin only.
- Video/recording codecs come from GStreamer. The AppImage bundles the plugins installed on the
  CI runner (see `release.yml`); the `.deb` depends on them. Without `gst-libav` there is no
  H.264/AAC (MP4) playback, without `gst-plugins-good` no `MediaRecorder`.
- NVIDIA: the app turns off WebKit's DMA-BUF renderer (blank window on X11, protocol error or
  web process abort on Wayland). Do not launch it with `LIBGL_ALWAYS_SOFTWARE=1` /
  `WEBKIT_DISABLE_COMPOSITING_MODE=1`: everything is then painted on the CPU.
- Liquid glass: `backdrop-filter` is only painted by the DMA-BUF renderer. When it is off the app
  sets `window.__TEURTASK_DESKTOP__.backdropBlur = false` and the web app turns the glass into solid
  surfaces without the filter (`no-backdrop-blur` in `chatTheme.css`).

---

## CI/CD

This directory is a 1:1 mirror of [`teurtask-destkop-app`](https://github.com/mator0296/teurtask-destkop-app):
`teurtask-frontend`'s GitLab pipeline (`sync:desktop-app` job) pushes every change under
`src-tauri/**` on `main` to that repo via `git subtree push`. **Do not edit `.github/workflows/`
directly on GitHub** — it would be overwritten (or break the sync) on the next push; change it
here instead.

Each push to `main` on the GitHub side triggers `.github/workflows/release.yml`, which builds
Linux, Windows, and macOS (Apple Silicon) in parallel and publishes a GitHub Release tagged
`v<version>` (read from `tauri.conf.json`). Permanent download links:

```
https://github.com/mator0296/teurtask-destkop-app/releases/latest/download/<asset>
```

| OS | Assets |
|---|---|
| Linux | `.deb`, `.rpm`, `.AppImage` |
| Windows | `.msi`, `-setup.exe` |
| macOS | `.dmg` (Apple Silicon only) |

To ship a new desktop build: bump `version` in `tauri.conf.json`, merge to `main` in
`teurtask-frontend`, and the release publishes automatically — no manual steps.

---

## Installing unsigned builds (internal use)

Since the app is unsigned, the OS will warn on first run:

**Windows** — SmartScreen dialog: click "More info" → "Run anyway"

**macOS** — Gatekeeper blocks the `.app`: right-click → Open, then confirm. Or run:
```
xattr -dr com.apple.quarantine teurtask.app
```

**Linux** — AppImage: `chmod +x teurtask_*.AppImage && ./teurtask_*.AppImage`
         — Deb: `sudo dpkg -i teurtask_*.deb`

---

## Updating icons

Place a square 1024×1024 PNG logo at `public/img/icon.png` and run:
```
npm run tauri icon public/img/icon.png
```
<!-- test: verify GitHub App sync job -->
