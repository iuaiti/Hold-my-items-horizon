# Hold My Items 5.1.1 for Horizon (.hmod)

Content-only Horizon mod package for Minecraft 26.3.

## Included content

- 3D item models and textures for boats, buckets, flowers, and other items
- Item and particle textures
- Custom sounds
- Lua pose/model scripts under `assets/minecraft/holdmyitems/`

## Important limitation

This repository is packaged as a **content-only `.hmod`**. The former Rust native-mod implementation has been removed. First-person rendering hooks, runtime physics, weapon inspection, dynamic hand lighting, and in-game commands are not provided by this content-only package.

## Build

The GitHub Actions workflow builds the `.hmod` on Ubuntu x86_64 and uploads it as the `holdmyitems-hmod` artifact.

You can also build locally with the Horizon CLI:

```bash
cargo install --locked --git https://github.com/horizon-rust/horizon cargo-horizon-mod
cargo horizon-mod build
```
