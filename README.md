# Hold My Items 5.1.1 for Horizon (Rust Minecraft 26.3 — Nightly `c9d474d`)

Native Horizon mod (`.hnmod`) port of **Hold My Items (HMI) 5.1.1** pinned to Horizon Nightly `c9d474dd9116e84c243f8992d75c11314df56f06` (`nightly-source/2026-11-01-c9d474d`, Minecraft Java Edition 26.3).

## Features Ported
- **First-Person Hand & Item Overhaul (`HeldItemRendererMixin`)**: Renders the player's arm gripping held weapons, tools, 3D blocks, lanterns, torches, buckets, food, potions, bows, crossbows, tridents, spears, shields, and filled maps.
- **Spring-Damper Viewmodel Physics (`hand_pose.lua`, `hand_relative_pose.lua`, `item_pose.lua`)**: Pitch/yaw inertia, vertical fall/jump spring-damper, walking/swimming/crawling/climbing/sneaking bobbing, off-hand auto-hide, and smooth hand-switch animations.
- **Per-Weapon Combat & Tool Choreography**: Independent per-hand swing progress (`LivingEntityMixin`), alternating 2-hit sword combo slashes, pickaxe/axe/mace/shovel/trident/spear jab & chop animations, and custom per-item swing durations (`trident`/`mace`: 12t, `shovels`: 14t, `spears`: 15t).
- **Per-Quad Model Articulation (`ModelPartAnimator` + `item_model.lua`)**: Liquid wobble in water/lava/milk buckets, animated axolotl/cod/salmon/tropical fish/pufferfish/tadpole buckets, and chest boat lid articulation.
- **Weapon Inspection (`C` key)**: Hold `C` (configurable in `Options > Mods > Hold My Items`) to inspect swords, pickaxes, axes, tridents, and maces with a 360-degree spin.
- **First-Person Particles & Dynamic Hand Lighting**: Soft additive glow on held torches, soul torches, redstone torches, lanterns, lava buckets, and brewing stands, petal/firefly bursts when swinging pink petals, wildflowers, or leaf litter, plus camera-local colored dynamic lights via `native_visuals`.
- **In-Game Settings & `/hmi` Command**: Configure in `Options > Mods > Hold My Items` or via `/hmi` (`/hmi reload`, `/hmi inspect`, `/hmi speed <ticks>`, `/hmi offset <x> <y> <z>`, `/hmi reset`).

## Installation (Horizon Nightly `c9d474d`)
1. Copy `holdmyitems-5.1.1.hnmod` into your Horizon instance's `horizon-native-mods/` folder (or run `horizon-launcher native-build holdmyitems-5.1.1.hnmod --trust-native-code`).
2. (Optional) Copy `holdmyitems-pack.zip` into your Horizon instance's `resourcepacks/` folder and enable it in `Options > Resource Packs` for the 3D fish bucket, chest boat, and flower models.
3. Click **Build** in the Horizon Launcher's Mods tab (`Native build`).
