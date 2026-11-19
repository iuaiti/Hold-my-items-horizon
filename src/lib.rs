// Hold My Items 5.1.1 — Horizon API 0.2 (WASM / Native) adapter.
// Wraps the full 1:1 Rust port (`src/runtime.rs`) onto `mod_api::Hooks`:
//   - `render_hand`: cancels vanilla first-person hand and renders the 3D player arm +
//     HMI-posed held item / 3D block using `HmiState::evaluate_hand`.
//   - `client_tick_post`: 20 TPS per-hand swing progression (`HmiState::tick_swings`).
//   - `camera_shake`: eating/drinking/walking/landing camera feedback (`HmiState::camera_shake`).
//   - `visuals`: dynamic colored hand lighting (`HmiState::hand_light_for_item`).
//   - `input_key`: Inspect keybind (`C` / `key.keyboard.c`) for weapon inspection.

#![allow(dead_code)]

mod runtime;

use std::sync::Mutex;

use mod_api::{
    self, BlockEntityDrawContext, CameraShakeEvent, ClientHost, ClientTickPostEvent, ColoredLight,
    CubeListBuilder, Flow, Hand, Hooks, InputKeyEvent, ItemDisplayContext, ItemStack, KeyAction,
    LayerDefinition, LivingRef, MeshDefinition, ModelPart, PartPose, RenderHandEvent, VisualsEvent,
};
use runtime::{Config, HmiState, ItemFlags, Mat4f, PlayerFrameInput, UseKind};

const LUA_HAND_POSE: &str = include_str!("../lua-scripts/hand_pose.lua");
const LUA_HAND_RELATIVE_POSE: &str = include_str!("../lua-scripts/hand_relative_pose.lua");
const LUA_ITEM_POSE: &str = include_str!("../lua-scripts/item_pose.lua");
const LUA_ITEM_MODEL: &str = include_str!("../lua-scripts/item_model.lua");

struct AdapterState {
    config: Config,
    hmi: HmiState,
    initialized: bool,
    last_anim_time: f64,
    inspect_down: bool,
    last_main_swing_raw: f32,
    last_off_swing_raw: f32,
    last_main_equip: f32,
    last_off_equip: f32,
    last_main_flags: ItemFlags,
    last_off_flags: ItemFlags,
    arm_right: ModelPart,
    arm_left: ModelPart,
}

impl Default for AdapterState {
    fn default() -> Self {
        let mut hmi = HmiState::default();
        for script in [
            LUA_HAND_POSE,
            LUA_HAND_RELATIVE_POSE,
            LUA_ITEM_POSE,
            LUA_ITEM_MODEL,
        ] {
            hmi.overrides.parse_script(script);
        }
        Self {
            config: Config::default(),
            hmi,
            initialized: false,
            last_anim_time: 0.0,
            inspect_down: false,
            last_main_swing_raw: 0.0,
            last_off_swing_raw: 0.0,
            last_main_equip: 0.0,
            last_off_equip: 0.0,
            last_main_flags: ItemFlags::empty(),
            last_off_flags: ItemFlags::empty(),
            arm_right: build_player_arm(false),
            arm_left: build_player_arm(true),
        }
    }
}

/// Builds a vanilla 64x64 player arm `ModelPart` (`4x12x4` texels) matching
/// `PlayerModel.rightArm` (`[-3, -2, -2]`, UV `(40, 16)`) and `leftArm` (`[-1, -2, -2]`, UV `(32, 48)`).
fn build_player_arm(left: bool) -> ModelPart {
    let mut mesh = MeshDefinition::new();
    let root = mesh.root();
    let (origin, u, v) = if left {
        ([-1.0, -2.0, -2.0], 32, 48)
    } else {
        ([-3.0, -2.0, -2.0], 40, 16)
    };
    let cubes = CubeListBuilder::create()
        .tex_offs(u, v)
        .add_box(origin, [4.0, 12.0, 4.0]);
    mesh.add_or_replace_child(root, "arm", cubes, PartPose::ZERO);
    LayerDefinition::create(mesh, 64, 64).bake_root()
}

/// Vanilla `ItemInHandRenderer.renderPlayerArm(equip = 0, swing = 0, left)` matrix
/// in first-person hand view space (`+x` right, `+y` up, `-z` into screen).
fn vanilla_player_arm_pose(left: bool) -> Mat4f {
    let f = if left { -1.0_f32 } else { 1.0_f32 };
    let mut m = Mat4f::IDENTITY;
    m.translate(f * 0.64000005, -0.6, -0.71999997);
    m.rotate_y(f * 45.0);
    m.translate(f * -1.0, 3.6, 3.5);
    m.rotate_z(f * 120.0);
    m.rotate_x(200.0);
    m.rotate_y(f * -135.0);
    m.translate(f * 5.6, 0.0, 0.0);
    m
}

/// Decomposes a column-major affine `Mat4f` into translation, axis-angle rotation (degrees),
/// and scale, and pushes it onto `BlockEntityDrawContext`.
fn apply_mat4(draw: &mut dyn BlockEntityDrawContext, m: Mat4f) {
    let d = m.0;
    let tx = d[12];
    let ty = d[13];
    let tz = d[14];
    if tx.is_finite() && ty.is_finite() && tz.is_finite() {
        draw.translate([tx, ty, tz]);
    }

    let c0 = [d[0], d[1], d[2]];
    let c1 = [d[4], d[5], d[6]];
    let c2 = [d[8], d[9], d[10]];

    let mut sx = (c0[0] * c0[0] + c0[1] * c0[1] + c0[2] * c0[2]).sqrt();
    let sy = (c1[0] * c1[0] + c1[1] * c1[1] + c1[2] * c1[2]).sqrt();
    let sz = (c2[0] * c2[0] + c2[1] * c2[1] + c2[2] * c2[2]).sqrt();

    if sx < 1e-6 || sy < 1e-6 || sz < 1e-6 {
        return;
    }

    // Preserve reflection sign if determinant is negative
    let det = c0[0] * (c1[1] * c2[2] - c1[2] * c2[1])
        - c1[0] * (c0[1] * c2[2] - c0[2] * c2[1])
        + c2[0] * (c0[1] * c1[2] - c0[2] * c1[1]);
    if det < 0.0 {
        sx = -sx;
    }

    let m00 = c0[0] / sx;
    let m10 = c0[1] / sx;
    let m20 = c0[2] / sx;
    let m01 = c1[0] / sy;
    let m11 = c1[1] / sy;
    let m21 = c1[2] / sy;
    let m02 = c2[0] / sz;
    let m12 = c2[1] / sz;
    let m22 = c2[2] / sz;

    // 3x3 rotation matrix -> unit quaternion (Shepperd's method)
    let trace = m00 + m11 + m22;
    let (qx, qy, qz, qw) = if trace > 0.0 {
        let s = (trace + 1.0).max(0.0).sqrt() * 2.0;
        ((m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, 0.25 * s)
    } else if m00 > m11 && m00 > m22 {
        let s = (1.0 + m00 - m11 - m22).max(1e-12).sqrt() * 2.0;
        (0.25 * s, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s)
    } else if m11 > m22 {
        let s = (1.0 + m11 - m00 - m22).max(1e-12).sqrt() * 2.0;
        ((m01 + m10) / s, 0.25 * s, (m12 + m21) / s, (m02 - m20) / s)
    } else {
        let s = (1.0 + m22 - m00 - m11).max(1e-12).sqrt() * 2.0;
        ((m02 + m20) / s, (m12 + m21) / s, 0.25 * s, (m10 - m01) / s)
    };

    let v_len = (qx * qx + qy * qy + qz * qz).sqrt();
    if v_len > 1e-5 {
        let angle_deg = 2.0 * v_len.atan2(qw).to_degrees();
        if angle_deg.is_finite() && angle_deg.abs() > 1e-4 {
            draw.rotate_about([qx / v_len, qy / v_len, qz / v_len], angle_deg, [0.0; 3]);
        }
    }

    if sx.is_finite() && sy.is_finite() && sz.is_finite() {
        draw.scale([sx, sy, sz]);
    }
}

fn build_item_flags(host: &dyn ClientHost, stack: Option<&ItemStack>) -> ItemFlags {
    let Some(stack) = stack else {
        return ItemFlags::empty();
    };
    let id = stack.item().id(host);
    if id.is_empty() {
        return ItemFlags::empty();
    }
    let short = id.strip_prefix("minecraft:").unwrap_or(&id);
    let is_block = host.block_key(&id).is_some();

    let has_kinetic = host
        .item_default_component(stack.item(), "minecraft:kinetic_weapon")
        .is_some();
    let has_blocks_attacks = host
        .item_default_component(stack.item(), "minecraft:blocks_attacks")
        .is_some();
    let has_consumable = host
        .item_default_component(stack.item(), "minecraft:consumable")
        .is_some();

    let use_kind = if has_kinetic || short == "spear" || short.ends_with("_spear") {
        UseKind::Spear
    } else if short == "bow" {
        UseKind::Bow
    } else if short == "crossbow" {
        UseKind::Crossbow
    } else if short == "trident" {
        UseKind::Trident
    } else if short == "brush" {
        UseKind::Brush
    } else if short == "spyglass" {
        UseKind::Spyglass
    } else if short == "goat_horn" {
        UseKind::TootHorn
    } else if has_blocks_attacks || short == "shield" {
        UseKind::Block
    } else if short == "potion" || short == "honey_bottle" || short == "milk_bucket" {
        UseKind::Drink
    } else if has_consumable {
        UseKind::Eat
    } else {
        UseKind::None
    };

    let is_charged_crossbow = short == "crossbow"
        && stack
            .components()
            .and_then(|c| c.as_nbt().get(&"minecraft:charged_projectiles".into()))
            .is_some();

    let is_cooling_down = use_kind == UseKind::Block
        && host
            .local_player()
            .and_then(|p| p.has_item_cooldown(host, &id))
            .unwrap_or(false);

    ItemFlags {
        id,
        is_empty: false,
        is_block,
        use_kind,
        is_charged_crossbow,
        is_cooling_down,
        spear_can_damage: true,
        spear_can_dismount: true,
        spear_can_knockback: true,
        spear_hit_impact: false,
    }
}

fn build_frame_input(
    host: &dyn ClientHost,
    partial: f32,
    inspect_down: bool,
    main_equip: f32,
    off_equip: f32,
) -> PlayerFrameInput {
    let Some(player) = host.local_player() else {
        return PlayerFrameInput::default();
    };
    let entity = player.entity();
    let living = LivingRef::new(entity);

    let [vx, vy, vz] = entity.velocity(host).unwrap_or([0.0; 3]);
    let speed = ((vx * vx + vz * vz).sqrt() as f32).clamp(0.0, 2.0);
    let y_speed = (vy as f32).clamp(-4.0, 4.0);
    let [yaw, pitch] = entity.rotation(host).unwrap_or([0.0, 0.0]);
    let on_ground = entity.on_ground(host).unwrap_or(true);
    let sneaking = entity.is_crouching(host).unwrap_or(false);
    let touching_water = entity.is_in_water(host).unwrap_or(false);
    let pose = entity.pose(host).unwrap_or("standing");
    let swimming = pose == "swimming" && touching_water;
    let crawling = pose == "swimming" && !touching_water;
    let using_riptide = pose == "spin_attack";
    let age_ticks = entity.tick_count(host).unwrap_or(0) as f32 + partial;

    let using_item = living.item_use_remaining(host).unwrap_or(0) > 0;
    let active_main_hand = living.used_item_hand(host) != Some(Hand::Off);

    PlayerFrameInput {
        pitch,
        yaw,
        speed,
        y_speed,
        on_ground,
        sneaking,
        swimming,
        crawling,
        climbing: false,
        touching_water,
        using_riptide,
        using_item,
        active_main_hand,
        inspect_held: inspect_down && !host.is_paused(),
        block_breaking: false,
        age_ticks,
        main_equip_progress: main_equip,
        off_equip_progress: off_equip,
    }
}

#[derive(Default)]
pub struct HoldMyItemsMod {
    state: Mutex<AdapterState>,
}

#[mod_api::horizon_mod]
impl Hooks for HoldMyItemsMod {
    fn render_hand(&self, host: &dyn ClientHost, event: &mut RenderHandEvent<'_, '_>) -> Flow {
        let Ok(mut state) = self.state.lock() else {
            return Flow::Continue;
        };

        let main_hand = matches!(event.render.hand, Hand::Main);
        let left = !main_hand;
        let right_arm = !left;
        let l = if right_arm { 1.0_f32 } else { -1.0_f32 };

        // Read both hands from player so two-handed / offhand interactions match HMI 5.1.1
        let (main_stack_owned, off_stack_owned) = if let Some(player) = host.local_player() {
            let living = LivingRef::new(player.entity());
            (
                living.hand_item(host, Hand::Main),
                living.hand_item(host, Hand::Off),
            )
        } else {
            (None, None)
        };

        let main_flags = if main_hand {
            build_item_flags(host, event.render.stack)
        } else {
            build_item_flags(host, main_stack_owned.as_ref())
        };
        let off_flags = if !main_hand {
            build_item_flags(host, event.render.stack)
        } else {
            build_item_flags(host, off_stack_owned.as_ref())
        };

        state.last_main_flags = main_flags.clone();
        state.last_off_flags = off_flags.clone();

        // Track equip progress & detect vanilla swing onset per hand
        let equip = event.render.equip.clamp(0.0, 1.0);
        let swing_raw = event.render.swing.clamp(0.0, 1.0);
        if main_hand {
            state.last_main_equip = equip;
            if swing_raw > 0.01 && (state.last_main_swing_raw <= 0.01 || swing_raw < state.last_main_swing_raw - 0.25) {
                let cfg = state.config.clone();
                state.hmi.on_swing(true, &main_flags, &cfg);
            }
            state.last_main_swing_raw = swing_raw;
        } else {
            state.last_off_equip = equip;
            if swing_raw > 0.01 && (state.last_off_swing_raw <= 0.01 || swing_raw < state.last_off_swing_raw - 0.25) {
                let cfg = state.config.clone();
                state.hmi.on_swing(false, &off_flags, &cfg);
            }
            state.last_off_swing_raw = swing_raw;
        }

        let input = build_frame_input(
            host,
            event.render.partial,
            state.inspect_down,
            state.last_main_equip,
            state.last_off_equip,
        );

        // Advance HMI spring-damper physics once per frame using animation_time()
        let anim_time = event.draw.animation_time();
        if !state.initialized {
            state.initialized = true;
            state.last_anim_time = anim_time;
        } else {
            let dt_ticks = anim_time - state.last_anim_time;
            if dt_ticks.is_finite() && dt_ticks > 1e-5 && dt_ticks < 5.0 {
                state.last_anim_time = anim_time;
                let dt_sec = (dt_ticks / 20.0).clamp(0.0, 0.05) as f32;
                let cfg = state.config.clone();
                let _ = state.hmi.step_frame(
                    dt_sec,
                    event.render.partial,
                    &main_flags,
                    &off_flags,
                    &input,
                    &cfg,
                );
            }
        }

        let (this_flags, other_flags) = if main_hand {
            (&main_flags, &off_flags)
        } else {
            (&off_flags, &main_flags)
        };

        // Let vanilla render filled maps so map canvas textures remain crisp
        if this_flags.is_filled_map() {
            return Flow::Continue;
        }

        let eval = state.hmi.evaluate_hand(
            main_hand,
            right_arm,
            this_flags,
            other_flags,
            event.render.partial,
            &input,
            &state.config,
        );

        // 1. Render the 3D player arm gripping the item (or empty hand)
        //    Exact equivalent of `native.rs` lines 599-604: `eval.arm_pose * player_arm_pose(left, 0, 0)`
        let arm_mat = eval.arm_pose.mul(vanilla_player_arm_pose(left));
        let arm_part = if left {
            &state.arm_left
        } else {
            &state.arm_right
        };
        event.draw.push();
        apply_mat4(event.draw, arm_mat);
        let _ = event
            .draw
            .model_part(arm_part, "entity/player/wide/steve", 0xFFFFFF);
        event.draw.pop();

        // 2. Render the held 3D block or item using HMI's exact `eval.item_pose`
        if !eval.effective_empty {
            if let Some(stack) = event.render.stack {
                if eval.render_as_3d_block {
                    if let Some(block_key) = host.block_key(&this_flags.id) {
                        // Exact block transform from `native.rs` lines 620-637
                        let mut block_mat = eval.item_pose.mul(Mat4f::from_translation(
                            0.22 * l,
                            0.25,
                            0.2,
                        ));
                        let s_name = this_flags.short_name();
                        if s_name.contains("torch")
                            || this_flags.is_lantern()
                            || s_name == "frogspawn"
                        {
                            block_mat = block_mat
                                .mul(Mat4f::from_translation(-0.05 * l, 0.0, 0.0))
                                .mul(Mat4f::from_scale(1.75, 1.75, 1.75));
                        } else {
                            block_mat =
                                block_mat.mul(Mat4f::from_translation(-0.25 * l, -0.05, 0.0));
                        }
                        if !right_arm {
                            block_mat =
                                block_mat.mul(Mat4f::from_translation(-0.3, 0.0, 0.0));
                        }
                        block_mat = block_mat
                            .mul(Mat4f::from_scale(0.3, 0.3, 0.3))
                            .mul(Mat4f::from_translation(-0.9 * l, -0.45, -0.7));

                        event.draw.push();
                        apply_mat4(event.draw, block_mat);
                        let drew = event.draw.block(block_key, &[]);
                        event.draw.pop();

                        if !drew {
                            // Fallback for block-entity blocks (chests, beds, banners, shulkers)
                            let (display, seed) = if left {
                                (ItemDisplayContext::ThirdPersonLeftHand, 1)
                            } else {
                                (ItemDisplayContext::ThirdPersonRightHand, 2)
                            };
                            event.draw.push();
                            apply_mat4(event.draw, eval.item_pose);
                            let _ = event.draw.item(stack, display, None, seed);
                            event.draw.pop();
                        }
                    }
                } else {
                    // Exact `ThirdPersonRightHand` / `ThirdPersonLeftHand` display context from `native.rs` line 805
                    let (display, seed) = if left {
                        (ItemDisplayContext::ThirdPersonLeftHand, 1)
                    } else {
                        (ItemDisplayContext::ThirdPersonRightHand, 2)
                    };
                    event.draw.push();
                    apply_mat4(event.draw, eval.item_pose);
                    let _ = event.draw.item(stack, display, None, seed);
                    event.draw.pop();
                }
            }
        }

        // Cancel vanilla first-person hand geometry so only HMI's arm + item draw
        Flow::Cancel
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPostEvent) {
        if host.is_paused() {
            return;
        }
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let main_flags = state.last_main_flags.clone();
        let off_flags = state.last_off_flags.clone();
        let cfg = state.config.clone();
        state.hmi.tick_swings(&main_flags, &off_flags, None, None, &cfg);
    }

    fn camera_shake(&self, host: &dyn ClientHost, event: &mut CameraShakeEvent<'_>) {
        if host.is_paused() {
            return;
        }
        let Ok(state) = self.state.lock() else {
            return;
        };
        let (pitch, yaw, roll, offset) = state.hmi.camera_shake(&state.config);
        event.shake.pitch += pitch;
        event.shake.yaw += yaw;
        event.shake.roll += roll;
        event.shake.offset[0] += offset[0];
        event.shake.offset[1] += offset[1];
        event.shake.offset[2] += offset[2];
    }

    fn visuals(&self, host: &dyn ClientHost, event: &mut VisualsEvent<'_>) {
        let Ok(state) = self.state.lock() else {
            return;
        };
        if !state.config.dynamic_lights {
            return;
        }
        let Some(player) = host.local_player() else {
            return;
        };
        let Some(eye) = player.entity().eye_position(host) else {
            return;
        };
        let main_light = runtime::hand_light_for_item(&state.last_main_flags.id);
        let off_light = runtime::hand_light_for_item(&state.last_off_flags.id);
        if let Some((rgb, radius, intensity)) = main_light.or(off_light) {
            event
                .lights
                .push(ColoredLight::new(eye, rgb, radius, intensity));
        }
    }

    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        if event.key == "key.keyboard.c" || event.key == "KeyC" || event.key == "c" {
            if let Ok(mut state) = self.state.lock() {
                state.inspect_down =
                    matches!(event.action, KeyAction::Press | KeyAction::Repeat) && !host.is_paused();
            }
        }
    }
}
