# Slope Orchard (斜坡果园)

T1, T2 and T5 start in **Prep**. Drag seeds onto hooks to choose species, move plants between hooks, or return a plant to the seed tray. T1 and T2 start with four plants and six usable hooks. Select your companions and press **开始守卫**. During battle, click fruit to drop it vertically or swipe across vines for angled cuts. The T2 reinforcement pauses combat until you plant one more seed on an empty hook.

T1 introduces the wild Hellhound. Feed it apples to recruit it. In T2 and T5, 小黑 herds monsters, kicks parked pumpkins and heads spent melons back toward the vines. Swipe a ripe apple into its mouth to start a downhill dash, or wake it from a nap. In T5, 露露西 converts puddles into fragile oil. A purple diamond marks a monster whose next fruit hit deals double damage.

T5 Prep and Sandbox share ten two-plant recipes. Drag a seed or a plant over another plant to preview the hybrid, then release to synthesize it. Either order gives the same fruit. Right-click a hybrid to split it or a single plant to clear it. Drag a plant to the seed tray to clear the whole hook. Hold Shift to swap hooks. In Sandbox, use right-drag for plant grafting and left-click to cut fruit.

| Parents | Fruit | Mature or giant harvest |
| --- | --- | --- |
| Berry + Pumpkin | 块魂滚怪球 | Carries slimes downhill. A heavy or a full ball scatters the passengers into a slime pool. |
| Apple + Pumpkin | 套娃皮纳塔瓜 | Bursts into a fan of apples on parking, breaking or Home Run. 小黑 can eat the apples. |
| Melon + Pumpkin | 打桩震地瓜 | Slams the ground, launches monsters and dents lids. A giant slam removes lids. |
| Berry + Melon | 吞怪大泡泡 | Swallows a monster, rebounds to chime height, then pops into a slime pool. |
| Apple + Melon | 三连弹珠串 | Splits on cutting into three fast marbles, or five when giant. |
| Apple + Berry | 糖渍诱饵果 | Sticks on landing and draws monsters in for five bites, or eight when giant. |
| Two apples | 双生红苹果 | Drops two apples per cut. |
| Two melons | 王冠甜瓜 | Rebounds higher and retains more energy. |
| Two pumpkins | 重甲巨南瓜 | Parks on the fifth heavy contact when giant. |
| Two berries | 巨泡粘液莓 | Spreads a slime pool twice as wide. |

Right-click near a falling ripe fruit or a parked pumpkin to launch a Flash-Step Home Run. The bar by the doorway fills over seven seconds. Swipe a giant apple left from the first hook to heal one heart, with a fifteen-second digestion cooldown. After defeat, **重新布阵** returns to Prep and **直接重试** restarts Battle with your current plants and companions.

## Windows

Run from this directory with Rust 1.100.0 MSVC and Godot 4.7.2 installed.

```powershell
cargo test --workspace --locked --offline
cargo run --release --bin balance_sim
python scripts/check_font.py
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
