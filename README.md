# Hold My Items 5.1.1 for Horizon (Minecraft 26.3)

This repository produces **two separate deliverables**:
> **Don't use it for now, it's just a draft.**

1. `holdmyitems.hmod` — a content-only Horizon mod containing files in the mod's own `holdmyitems` namespace.
2. `holdmyitems-resourcepack.zip` — the original Minecraft resource-pack overrides, preserving the `assets/minecraft/` paths required for vanilla item models, textures, sounds, and OptiFine emissive properties.

## Why these are separate

Horizon content mods cannot claim arbitrary files in the `minecraft` namespace. The resource-pack overrides therefore remain in a standalone resource pack instead of being forced into the `.hmod` package or renamed (which would break vanilla resource locations).

The Lua scripts are retained in the resource pack at their original `assets/minecraft/holdmyitems/` paths and also included under the mod's owned namespace in the `.hmod`.

## Contents

- `resourcepack/assets/minecraft/` — vanilla item definitions, models, textures, particles, sounds, OptiFine emissive properties, and Lua scripts.
- `assets/holdmyitems/holdmyitems/` — Lua scripts included in the Horizon mod's own namespace.
- `lua-scripts/` — editable source copies of the Lua scripts.

## Build

The GitHub Actions workflow builds both deliverables on Ubuntu x86_64:

- `holdmyitems-hmod` — contains `holdmyitems.hmod`.
- `holdmyitems-resourcepack` — contains `holdmyitems-resourcepack.zip`.

Install the ZIP as a normal Minecraft resource pack. Enable it in the resource-pack settings. The `.hmod` is installed separately through Horizon.

## Important limitation

This is a content/resource-pack port, not the former native runtime implementation. First-person rendering hooks, runtime physics, weapon inspection, dynamic hand lighting, and in-game commands are not supplied by these packages.
