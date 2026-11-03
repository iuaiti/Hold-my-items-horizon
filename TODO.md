# HMI Runtime TODO

Tracking the runtime and input issues identified while porting Hold My Items 5.1.1 to Horizon.

## Swing animation

- [ ] Fix repeated swings caused by progress interpolation crossing a reset boundary.
  - Review `main_swing`, `off_swing`, and `on_swing` in `runtime.rs`.
  - Reset both current and previous swing progress when starting a new swing; do not treat a reset as a wrapped progress cycle.
- [ ] Prevent swing progress from being integrated twice by the 20 TPS tick path and the render-frame path.
  - Keep gameplay tick responsibilities separate from frame-time (`dt`) swing integration.
  - Preserve standalone unit-test behavior for `tick_swings`.
- [ ] Distinguish a continued button hold from a fresh click.
  - A normal 60–100 ms click crossing a tick boundary must trigger exactly one swing.
  - Fresh clicks, including rapid/butterfly clicks, must be accepted without interrupting a continuous block-breaking cycle.
  - Continuous mining must complete its swing cycle without mid-cycle truncation or unintended combo reversal.

## Mouse and mining state

- [ ] Prevent stuck mouse/mining state when opening a GUI, menu, or dialog while a button is held.
- [ ] Handle Creative mode correctly when the held item cannot break blocks, without leaving the destroying state active.
- [ ] Preserve Toggle Attack and multiplayer / `--server-thread` behavior.
- [ ] Centralize the breaking-state check in `is_hmi_breaking_block()`, accounting for mining state, left-button state, focus, and mouse capture.
- [ ] Run mouse/mining state sanitization at the appropriate tick and frame boundaries.
- [ ] Load the bundled HMI resource pack once before entering the world; avoid triggering a pack reload on the first in-world frame.

## Held-item equip animation

- [ ] Correct the equip-height expression in `main_equip` / `off_equip`:
  - Use `(1.0 - height) * scale`, not `1.0 - height * scale`.
- [ ] Prevent `equip_progress` from triggering an unintended downward hand motion after cooldown updates, durability changes, or inventory synchronization.
- [ ] Once the hand-switch animation has completed (`hand_switch_val >= 1.0`), keep equip progress at zero unless an actual item/slot change or item-use transition requires an update.

## Long-session numerical stability

- [ ] Bound accumulated animation phase values (`a`, `tilting`, `walk`, `water_walk`, `swim_counter`, `crawler`, and `crwl`) modulo `TAU`.
- [ ] Replace potentially unbounded yaw-normalization loops with `rem_euclid(360.0)`.
- [ ] Disable food-speed animation when the food counter is effectively zero.

## Verification still required

- [ ] Build the native mod against the target Horizon revision with `horizon-launcher native-build`.
- [ ] Verify single-click behavior across a tick boundary (including an approximately 80 ms click).
- [ ] Verify rapid clicking / 10–15 CPS and sword combo alternation.
- [ ] Verify continuous mining, Toggle Attack, Creative mode, GUI focus changes, and multiplayer / `--server-thread`.
- [ ] Run a longer play session to check for recurring swing, input, or floating-point issues.

Static inspection and mathematical simulation alone do not establish that the native build or in-game runtime behavior passes.
