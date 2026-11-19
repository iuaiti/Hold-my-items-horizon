# Hold My Items 5.1.1 for Horizon (`.hmod` — Rust Minecraft 26.3)

Horizon `.hmod` port of **Hold My Items (HMI) 5.1.1**, migrated from the retired `.hnmod` (`horizon-mixins`) bundle layout to Horizon's unified `.hmod` project structure.

## What Changed in the `.hmod` Port
- **Unified `.hmod` Layout**: Replaced the retired `horizon-native.json` and `mixins/holdmyitems/horizon-mixin.toml` bundle with a root `horizon-mod.toml`, flat `Cargo.toml` (`src/lib.rs` + `src/runtime.rs`), and native `assets/` directory.
- **Built-In `assets/` (No Separate Resource Pack Zip)**: Unpacked the 3D fish buckets, chest boats, flower baskets, particles, sounds, and Lua pose scripts directly into `assets/minecraft/`, removing the need to manually install `holdmyitems-pack.zip` into `resourcepacks/`.
- **Fixed Wheat-in-Hand Bug (`minecraft:wheat`)**:
  - **Root cause**: `minecraft:wheat` shares its registry ID with the `minecraft:wheat` crop block (`Blocks.WHEAT`), even though harvested wheat (`Items.WHEAT`) is a regular item (only `minecraft:wheat_seeds` places the crop). Checking `registry.default_state(&id).is_some()` misclassified held wheat as a 3D block (`is_block = true`), rendering the 3D `age=0` wheat crop model and applying block hand poses.
  - **Fix**: Excluded `"wheat"` from `is_block` classification (`src/native_legacy.rs`), added `"wheat"` to `ItemFlags::should_render_as_block()` exclusions (`src/runtime.rs`) with a unit test (`wheat_is_not_rendered_as_block`), and added `renderAsBlock:put("minecraft:wheat", false)` to `item_pose.lua` (both `lua-scripts/` and `assets/minecraft/holdmyitems/`).

## Current Upstream Horizon SDK Status (`horizon-mod-api`)
Horizon retired `.hnmod` and now compiles native `.hmod` crates against the prebuilt `horizon-mod-api` SDK (`.horizon/sdk/api/crates/horizon-mod-api`) without rebuilding the game or running `horizon-mixins` injections into private `Game<'_>` internals.

- **Ready today**:
  - `assets/minecraft/` (3D models, textures, sounds, and Lua scripts) works directly as a `.hmod` asset layer.
  - `src/runtime.rs` (4,280+ lines of spring-damper physics, pose math, swing state machine, per-quad `ModelPartAnimator`, config, and Lua override parser) compiles and tests cleanly as standalone Rust (`cargo test`).
- **Blocked on upstream `horizon-mod-api` hooks**:
  - The legacy `horizon-mixins` bridge (`src/native_legacy.rs`) relied on internal injections into `Game::item_activation_geometry` and `Game::swing_hand` plus direct access to private `Game<'_>` fields (`pack.item_renderer`, `block_model_vertices`, `special_block_model`, `resolve_item`, `gui.atlas.sprites`, `Vec<Vertex>`).
  - Horizon's `horizon-mod-api` does not yet provide a client-side first-person held-item / hand geometry render hook (tracked upstream in `horizon-rust/horizon#10074`). Once Horizon exposes first-person hand rendering and swing hooks in `horizon-mod-api`, wire `src/lib.rs` to `src/runtime.rs` using `src/native_legacy.rs` as reference.

## Project Layout
```text
Hold-my-items-horizon-hmod/
├── horizon-mod.toml          # Unified .hmod manifest (run `cargo horizon-mod update` to pin SDK)
├── Cargo.toml                # Flat native crate manifest
├── src/
│   ├── lib.rs                # .hmod crate root
│   ├── runtime.rs            # Pure-Rust HMI physics, poses, swing logic, Lua overrides & unit tests
│   └── native_legacy.rs      # Former horizon-mixins Game<'_> bridge (porting reference, wheat fixed)
├── assets/minecraft/         # Native .hmod 3D models, textures, sounds, and holdmyitems/*.lua
├── lua-scripts/              # Source Lua pose/model scripts (synced with assets/)
├── README.md
├── TODO.md
└── LICENSE
```

## Features
- **First-Person Hand & Item Overhaul**: Renders the player's arm gripping held weapons, tools, 3D blocks, lanterns, torches, buckets, food, potions, bows, crossbows, tridents, spears, shields, and filled maps.
- **Spring-Damper Viewmodel Physics (`hand_pose.lua`, `hand_relative_pose.lua`, `item_pose.lua`)**: Pitch/yaw inertia, vertical fall/jump spring-damper, walking/swimming/crawling/climbing/sneaking bobbing, off-hand auto-hide, and smooth hand-switch animations.
- **Per-Weapon Combat & Tool Choreography**: Independent per-hand swing progress, alternating 2-hit sword combo slashes, pickaxe/axe/mace/shovel/trident/spear jab & chop animations, and custom per-item swing durations (`trident`/`mace`: 12t, `shovels`: 14t, `spears`: 15t).
- **Per-Quad Model Articulation (`ModelPartAnimator` + `item_model.lua`)**: Liquid wobble in water/lava/milk buckets, animated axolotl/cod/salmon/tropical fish/pufferfish/tadpole buckets, and chest boat lid articulation.
- **Weapon Inspection (`C` key)**: Hold `C` (configurable in `Options > Mods > Hold My Items`) to inspect swords, pickaxes, axes, tridents, and maces with a 360-degree spin.
- **First-Person Particles & Dynamic Hand Lighting**: Soft additive glow on held torches, soul torches, redstone torches, lanterns, lava buckets, and brewing stands, petal/firefly bursts, and camera-local colored dynamic lights.
- **In-Game Settings & `/hmi` Command**: Configure in `Options > Mods > Hold My Items` or via `/hmi` (`/hmi reload`, `/hmi inspect`, `/hmi speed <ticks>`, `/hmi offset <x> <y> <z>`, `/hmi reset`).

## Development & Building (`.hmod`)
1. Install `cargo-horizon-mod` (requires Rust 1.95+):
   ```bash
   cargo install --locked --git https://github.com/horizon-rust/horizon cargo-horizon-mod
   ```
2. Run standalone physics & pose unit tests:
   ```bash
   cargo test
   ```
3. Pin to your target Horizon Nightly SDK and fetch `.horizon/sdk/`:
   ```bash
   cargo horizon-mod update
   cargo horizon-mod check
   ```
4. Build and install into a Horizon instance once `src/lib.rs` is wired to the upstream `horizon-mod-api` first-person hand hooks:
   ```bash
   horizon-launcher native-build . --instance "My Instance" --trust-native-code
   ```
