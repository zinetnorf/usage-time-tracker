# Development

## Prerequisites

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) 22+
- [pnpm](https://pnpm.io) 11+
- Linux is not supported yet (tracking uses Windows/macOS APIs).

## Commands

```bash
pnpm install        # frontend dependencies
pnpm tauri dev      # app in development mode (hot reload)

# Tests
cd src-tauri && cargo test    # Rust core
pnpm exec vitest run          # frontend utilities

# Production build
pnpm tauri build                                      # current platform
pnpm tauri build --target universal-apple-darwin      # universal macOS
```

Installers are produced under `src-tauri/target/**/release/bundle/`.

For the universal macOS build you need both targets installed:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

## Project layout

```
src/                  # React dashboard (views, lib, i18n)
src-tauri/src/
  db.rs               # SQLite: schema, migrations, queries (TDD)
  tracker.rs          # tracking state machine (TDD)
  platform.rs         # app identity, session lock, accessibility (per-OS)
  runloop.rs          # tracking thread: poll → tick → persist
  tray.rs             # tray icon and menu
  commands.rs         # Tauri commands exposed to the webview
  lib.rs              # app setup: plugins, state, window events
scripts/gen_icon.py   # source icon generator (1024x1024)
```

## Automated releases

The GitHub Actions workflow (`.github/workflows/release.yml`) builds macOS (universal) and Windows when a tag is pushed:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

It creates a **draft release** with the installers attached; review and publish it from GitHub. Version bumps go in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `package.json` before tagging.
