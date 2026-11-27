# Slope Orchard (斜坡果园)

Defend the cabin through four waves. Hover to line up a drop, then click a fruit or hold the left mouse button and swipe across several. Wait for fruit to ripen for stronger hits, or cut early to catch an enemy under a hook. Overripe fruit drops on its own.

Right-click near a falling ripe fruit or a parked pumpkin to launch a Flash-Step Home Run. The bar by the doorway fills as the seven-second cooldown expires. Swipe left across the first hook to feed the protagonist a Giant Red Apple when he is hurt. Each apple restores one heart, with a 15-second digestion period between heals.

Between waves, drag plants between hooks and move chimes to the gaps. The final grant adds a seventh plant to the reserve, which you can swap with any of the six planted hooks. Select 下一波 to resume. In 满配沙盒, enemies keep coming and you can rearrange plants while the simulation runs. Hold Shift when dragging to keep the Home Run from triggering.

## Windows

Run from this directory with Rust 1.100.0 MSVC and Godot 4.7.2 installed.

```powershell
cargo test --locked --offline
powershell -ExecutionPolicy Bypass -File .\scripts\build.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\run.ps1
```

The build script compiles and installs the release DLL, imports the Godot project, and runs a headless campaign and sandbox probe. Godot draws the scene and handles input and effects. The Rust core advances the orchard at 120 fixed simulation steps per second.

## Web

The Web preset uses the Godot 4.7.2 extensions-enabled, single-threaded template. Install the `rust-src` component and `wasm32-unknown-emscripten` target for the nightly toolchain, and put Emscripten 4.0.20 on PATH. The Web build uses immediate abort for compatibility with the template's WebAssembly exception ABI. [godot-rust WebAssembly compatibility discussion](https://github.com/godot-rust/gdext/issues/1119)

```powershell
$env:CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS = '-Z unstable-options -C panic=immediate-abort'
cargo +nightly-2026-11-21-x86_64-pc-windows-msvc build -Z build-std=std,panic_abort --release --locked --offline --target wasm32-unknown-emscripten -p slope-orchard-extension --no-default-features --features web
Remove-Item Env:CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS
Copy-Item target/wasm32-unknown-emscripten/release/slope_orchard_extension.wasm godot/native/
New-Item -ItemType Directory -Force dist/web
godot_console --headless --path godot --export-release Web ../dist/web/index.html
python -m http.server --directory dist/web 8000
```

Open `http://localhost:8000` in a WebGL 2 browser.
