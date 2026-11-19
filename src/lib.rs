//! Hold My Items 5.1.1 — Horizon `.hmod` crate root.
//!
//! - [`runtime`] contains the standalone, pure-Rust HMI spring-damper physics,
//!   hand/item pose evaluator, per-quad model articulation (`ModelPartAnimator`),
//!   configuration, and Lua override parser (runnable with `cargo test`).
//! - `src/native_legacy.rs` preserves the former `horizon-mixins` `Game<'_>`
//!   bridge (with the `minecraft:wheat` held-item fix applied) as a porting
//!   reference while Horizon's `horizon-mod-api` adds first-person held-item
//!   rendering hooks (see `horizon-rust/horizon#10074` and `TODO.md`).

pub mod runtime;

// Future `horizon-mod-api` entrypoint (enable once `horizon-mod-api` in
// `.horizon/sdk/api/crates/horizon-mod-api` exposes first-person held-item /
// hand geometry and per-hand swing hooks):
//
// pub struct HoldMyItemsMod {
//     state: runtime::HmiState,
// }
