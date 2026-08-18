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

---

## GitLab CI

The `.gitlab-ci.yml` at the repo root triggers on git tags and produces:
- **Linux:** `.deb` and `.AppImage`
- **Windows:** `.exe` (NSIS installer) — cross-compiled from Linux via `cargo-xwin`

Tag a release to trigger builds:
```
git tag v1.9.0 && git push origin v1.9.0
```

Artifacts are kept for 30 days under the pipeline's job artifacts.

**Required CI variable** (set in GitLab → Settings → CI/CD → Variables):
- `VITE_API_BASE_URL` — defaults to `https://api.teurtask.com` in `.gitlab-ci.yml`

---

## macOS build (local)

macOS cannot be cross-compiled. Build on your Mac:
```
npm ci
npm run tauri:build
```
Output: `src-tauri/target/release/bundle/dmg/` and `bundle/macos/`.

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

<!-- sync test 2: debian image fix -->
