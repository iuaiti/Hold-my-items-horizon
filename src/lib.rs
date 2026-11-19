use std::f32::consts::PI;
use std::sync::Mutex;

use mod_api::{
    self, ClientHost, CubeListBuilder, Flow, Hand, Hooks, LayerDefinition, LivingRef,
    MeshDefinition, ModelPart, PartPose, RenderHandEvent,
};

// Spring physics constants ported from Hold My Items 5.1.1 (item_pose.lua)
const GRAVITY: f32 = 0.04;
const DAMPING: f32 = 0.85;
const INTENSITY: f32 = 0.15;

#[derive(Default)]
struct HandSpring {
    pitch_speed: f32,
    pitch_angle: f32,
    yaw_speed: f32,
    yaw_angle: f32,
    switch_anim: f32,
    use_anim: f32,
    last_item_id: String,
    last_equip: f32,
}

struct AnimState {
    initialized: bool,
    last_anim_time: f64,
    prev_yaw: f32,
    prev_pitch: f32,
    idle_time: f32,
    walk_phase: f32,
    walk_smoother: f32,
    swim_counter: f32,
    swim_smoother: f32,
    fall_amount: f32,
    sneak_smoother: f32,
    main_hand: HandSpring,
    off_hand: HandSpring,
    arm_right: ModelPart,
    arm_left: ModelPart,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            initialized: false,
            last_anim_time: 0.0,
            prev_yaw: 0.0,
            prev_pitch: 0.0,
            idle_time: 0.0,
            walk_phase: 0.0,
            walk_smoother: 0.0,
            swim_counter: 0.0,
            swim_smoother: 0.0,
            fall_amount: 0.0,
            sneak_smoother: 0.0,
            main_hand: HandSpring::default(),
            off_hand: HandSpring::default(),
            arm_right: build_player_arm(false),
            arm_left: build_player_arm(true),
        }
    }
}

fn build_player_arm(left: bool) -> ModelPart {
    let mut mesh = MeshDefinition::new();
    let root = mesh.root();
    let (u, v) = if left { (32, 48) } else { (40, 16) };
    let cubes = CubeListBuilder::create()
        .tex_offs(u, v)
        .add_box([-2.0, -6.0, -2.0], [4.0, 12.0, 4.0]);
    mesh.add_or_replace_child(root, "arm", cubes, PartPose::ZERO);
    LayerDefinition::create(mesh, 64, 64).bake_root()
}

#[derive(Default)]
pub struct HoldMyItemsMod {
    state: Mutex<AnimState>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Empty,
    Map,
    Sword,
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    Mace,
    Trident,
    Spear,
    Bow,
    Crossbow,
    Shield,
    FoodOrDrink,
    TorchOrLantern,
    Bucket,
    ThrowableOrSmall,
    Bed,
    Generic,
}

fn classify_item(id: &str) -> ItemKind {
    if id.is_empty() {
        return ItemKind::Empty;
    }
    let name = id.strip_prefix("minecraft:").unwrap_or(id);
    if name == "filled_map" || name == "map" {
        ItemKind::Map
    } else if name.ends_with("_sword") {
        ItemKind::Sword
    } else if name.ends_with("_pickaxe") {
        ItemKind::Pickaxe
    } else if name.ends_with("_axe") {
        ItemKind::Axe
    } else if name.ends_with("_shovel") {
        ItemKind::Shovel
    } else if name.ends_with("_hoe") {
        ItemKind::Hoe
    } else if name == "mace" {
        ItemKind::Mace
    } else if name == "trident" {
        ItemKind::Trident
    } else if name.ends_with("_spear") || name == "spear" {
        ItemKind::Spear
    } else if name == "bow" {
        ItemKind::Bow
    } else if name == "crossbow" {
        ItemKind::Crossbow
    } else if name == "shield" {
        ItemKind::Shield
    } else if name == "potion"
        || name == "honey_bottle"
        || name == "milk_bucket"
        || name == "apple"
        || name == "golden_apple"
        || name == "enchanted_golden_apple"
        || name == "bread"
        || name == "cooked_beef"
        || name == "cooked_porkchop"
        || name == "cooked_chicken"
        || name == "cooked_mutton"
        || name == "cooked_rabbit"
        || name == "cooked_cod"
        || name == "cooked_salmon"
        || name == "baked_potato"
        || name == "golden_carrot"
        || name == "carrot"
        || name == "melon_slice"
        || name == "sweet_berries"
        || name == "glow_berries"
        || name == "cookie"
        || name == "pumpkin_pie"
        || name == "chorus_fruit"
        || name == "dried_kelp"
        || name == "mushroom_stew"
        || name == "rabbit_stew"
        || name == "beetroot_soup"
        || name == "suspicious_stew"
        || name == "beef"
        || name == "porkchop"
        || name == "chicken"
        || name == "mutton"
        || name == "rotten_flesh"
    {
        ItemKind::FoodOrDrink
    } else if name.contains("torch")
        || name.contains("lantern")
        || name.contains("candle")
        || name == "bell"
        || name == "end_crystal"
        || name.ends_with("_hanging_sign")
        || name == "pink_petals"
        || name == "leaf_litter"
        || name == "wildflowers"
    {
        ItemKind::TorchOrLantern
    } else if name.ends_with("_bucket") || name == "bucket" {
        ItemKind::Bucket
    } else if name == "ender_pearl"
        || name == "ender_eye"
        || name == "snowball"
        || name == "egg"
        || name == "wind_charge"
        || name == "experience_bottle"
        || name.starts_with("music_disc_")
        || name.ends_with("_bundle")
        || name == "bundle"
        || name.ends_with("_head")
        || name.ends_with("_skull")
        || name.ends_with("_nugget")
    {
        ItemKind::ThrowableOrSmall
    } else if name.ends_with("_bed") {
        ItemKind::Bed
    } else {
        ItemKind::Generic
    }
}

#[inline]
fn ease_custom(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    3.0 * t * (1.0 - t) * (1.0 - t) * 0.44 + 3.0 * t2 * (1.0 - t) + t3
}

#[inline]
fn ease_custom_sec(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    3.0 * t * (1.0 - t) * (1.0 - t) * 0.44 + 3.0 * t2 * (1.0 - t) * 0.94 + t3
}

#[inline]
fn ease_in_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    const C1: f32 = 1.70158;
    const C2: f32 = C1 * 1.525;
    if t < 0.5 {
        ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
    } else {
        ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0) / 2.0
    }
}

#[inline]
fn ease_in_out_sine(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    -(f32::cos(PI * t) - 1.0) * 0.5
}

#[inline]
fn wrap_angle_delta(delta: f32) -> f32 {
    let mut d = (delta + 180.0).rem_euclid(360.0) - 180.0;
    if d < -180.0 {
        d += 360.0;
    }
    d.clamp(-45.0, 45.0)
}

#[inline]
fn approach(current: f32, target: f32, rate: f32) -> f32 {
    current + (target - current) * rate.clamp(0.0, 1.0)
}

#[mod_api::horizon_mod]
impl Hooks for HoldMyItemsMod {
    fn render_hand(&self, host: &dyn ClientHost, event: &mut RenderHandEvent<'_, '_>) -> Flow {
        let Ok(mut state) = self.state.lock() else {
            return Flow::Continue;
        };

        let is_main = matches!(event.render.hand, Hand::Main);
        let side_sign: f32 = if is_main { 1.0 } else { -1.0 };

        // Read item id for the rendered hand
        let item_id = event
            .render
            .stack
            .map(|s| s.item().id(host))
            .unwrap_or_default();
        let kind = classify_item(&item_id);

        // Compute smooth frame-rate independent delta time (in seconds) from animation clock
        let anim_time = event.draw.animation_time();
        let dt_ticks = if state.initialized {
            let diff = anim_time - state.last_anim_time;
            if diff.is_finite() && diff >= 0.0 && diff < 5.0 {
                diff as f32
            } else {
                0.0
            }
        } else {
            0.0
        };
        let dt_sec = (dt_ticks / 20.0).clamp(0.0, 0.1);
        let step = dt_sec * 30.0; // Hold My Items normalizes physics at 30 Hz

        // Read local player state
        let mut speed_xz = 0.0_f32;
        let mut vel_y = 0.0_f32;
        let mut on_ground = true;
        let mut crouching = false;
        let mut sprinting = false;
        let mut in_water = false;
        let mut yaw = state.prev_yaw;
        let mut pitch = event.render.interpolated_pitch;
        let mut using_this_hand = false;
        let mut use_remaining = 0_i32;

        if let Some(player) = host.local_player() {
            let entity = player.entity();
            let living = LivingRef::new(entity);

            if let Some([vx, vy, vz]) = entity.velocity(host) {
                speed_xz = ((vx * vx + vz * vz).sqrt() as f32).clamp(0.0, 1.5);
                vel_y = (vy as f32).clamp(-3.0, 3.0);
            }
            if let Some([y, p]) = entity.rotation(host) {
                yaw = y;
                pitch = p;
            }
            on_ground = entity.on_ground(host).unwrap_or(true);
            crouching = entity.is_crouching(host).unwrap_or(false);
            sprinting = entity.is_sprinting(host).unwrap_or(false);
            in_water = entity.is_in_water(host).unwrap_or(false);

            if let Some(rem) = living.item_use_remaining(host) {
                if rem > 0 {
                    use_remaining = rem;
                    if living.used_item_hand(host) == Some(event.render.hand) {
                        using_this_hand = true;
                    }
                }
            }
        }

        // Update shared player-level motion once per frame (when anim_time advances)
        let mut dyaw = 0.0_f32;
        let mut dpitch = 0.0_f32;
        if !state.initialized {
            state.initialized = true;
            state.last_anim_time = anim_time;
            state.prev_yaw = yaw;
            state.prev_pitch = pitch;
        } else if step > 0.0001 {
            state.last_anim_time = anim_time;
            dyaw = wrap_angle_delta(yaw - state.prev_yaw);
            dpitch = wrap_angle_delta(pitch - state.prev_pitch);
            state.prev_yaw = yaw;
            state.prev_pitch = pitch;

            state.idle_time = (state.idle_time + dt_sec * 2.2) % (PI * 200.0);

            let moving = speed_xz > 0.015 && on_ground && !in_water;
            let walk_target = if moving {
                (speed_xz * 4.5).clamp(0.0, 1.0)
            } else {
                0.0
            };
            state.walk_smoother = approach(state.walk_smoother, walk_target, dt_sec * 10.0);
            let walk_freq = if sprinting { 11.5 } else { 8.0 };
            state.walk_phase =
                (state.walk_phase + dt_sec * walk_freq * (0.4 + state.walk_smoother)) % (PI * 200.0);

            let swim_target = if in_water { 1.0 } else { 0.0 };
            state.swim_smoother = approach(state.swim_smoother, swim_target, dt_sec * 6.0);
            state.swim_counter = (state.swim_counter + dt_sec * 4.5) % (PI * 200.0);

            let fall_target = if !on_ground {
                (-vel_y * 1.4).clamp(-1.2, 2.2)
            } else {
                0.0
            };
            state.fall_amount = approach(state.fall_amount, fall_target, dt_sec * 12.0);

            let sneak_target = if crouching { 1.0 } else { 0.0 };
            state.sneak_smoother = approach(state.sneak_smoother, sneak_target, dt_sec * 12.0);
        }

        // Snapshot shared values so we can mutably borrow per-hand spring cleanly
        let walk_phase = state.walk_phase;
        let walk_smoother = state.walk_smoother;
        let swim_counter = state.swim_counter;
        let swim_smoother = state.swim_smoother;
        let fall_amount = state.fall_amount;
        let sneak_smoother = state.sneak_smoother;
        let idle_time = state.idle_time;

        let swing_raw = event.render.swing.clamp(0.0, 1.0);
        let equip_raw = event.render.equip.clamp(0.0, 1.0);

        let (pitch_angle, yaw_angle, switch_anim, use_anim) = {
            let hs = if is_main {
                &mut state.main_hand
            } else {
                &mut state.off_hand
            };

            if hs.last_item_id != item_id || (equip_raw > 0.65 && hs.last_equip <= 0.65) {
                hs.last_item_id = item_id.clone();
                hs.switch_anim = 0.0;
            }
            hs.last_equip = equip_raw;

            if step > 0.0001 {
                hs.switch_anim = (hs.switch_anim + dt_sec * 2.6).min(1.0);
                let use_target = if using_this_hand { 1.0 } else { 0.0 };
                hs.use_anim = approach(hs.use_anim, use_target, dt_sec * 11.0);

                // Pitch & yaw damped harmonic spring (item_pose.lua lines 151-179)
                let pitch_drive = (-speed_xz * 22.0 * walk_smoother)
                    - (swing_raw * PI).sin() * 8.0
                    + fall_amount * 3.0
                    + (sneak_smoother * PI).sin() * 0.3
                    + dpitch;
                hs.pitch_speed += pitch_drive * INTENSITY * step;
                hs.pitch_speed -= GRAVITY * hs.pitch_angle * step;
                hs.pitch_speed *= DAMPING.powf(step);
                hs.pitch_angle = (hs.pitch_angle + hs.pitch_speed * step).clamp(-35.0, 35.0);

                let yaw_drive = walk_phase.sin() * 3.0 * walk_smoother
                    + (swing_raw * PI).sin() * 8.0
                    + (swim_counter * swim_smoother).sin() * 3.0
                    + (hs.switch_anim * 2.0 * PI).sin() * 2.0
                    + dyaw;
                hs.yaw_speed += yaw_drive * INTENSITY * step;
                hs.yaw_speed -= GRAVITY * hs.yaw_angle * step;
                hs.yaw_speed *= DAMPING.powf(step);
                hs.yaw_angle = (hs.yaw_angle + hs.yaw_speed * step).clamp(-35.0, 35.0);
            }

            (hs.pitch_angle, hs.yaw_angle, hs.switch_anim, hs.use_anim)
        };

        // Match the backup implementation: let vanilla render filled maps unchanged.
        if kind == ItemKind::Map {
            return Flow::Continue;
        }

        // Accumulate pose offsets
        let mut tx = 0.0_f32;
        let mut ty = 0.0_f32;
        let mut tz = 0.0_f32;
        let mut rx = 0.0_f32; // pitch (degrees)
        let mut ry = 0.0_f32; // yaw (degrees)
        let mut rz = 0.0_f32; // roll (degrees)

        // 1. Subtle idle breathing
        let breathe = idle_time.sin() * (1.0 - walk_smoother * 0.6);
        ty += breathe * 0.008;
        rx += breathe * 0.6;

        // 2. Walk / sprint bobbing + swim sway + sneak crouch tilt
        let walk_bob_x = walk_phase.cos() * 0.028 * walk_smoother * side_sign;
        let walk_bob_y = walk_phase.sin().abs() * -0.025 * walk_smoother;
        tx += walk_bob_x;
        ty += walk_bob_y;
        rz += walk_phase.cos() * 2.4 * walk_smoother * side_sign;

        if swim_smoother > 0.01 {
            tx += swim_counter.sin() * 0.03 * swim_smoother * side_sign;
            ty += swim_counter.cos() * 0.02 * swim_smoother;
            rz += swim_counter.sin() * 5.0 * swim_smoother * side_sign;
        }

        if sneak_smoother > 0.01 {
            ty -= 0.035 * sneak_smoother;
            rz += -4.0 * sneak_smoother * side_sign;
        }

        // 3. Spring-damper camera & movement inertia
        tx += (-yaw_angle * 0.0028).clamp(-0.12, 0.12);
        ty += (pitch_angle * 0.0022 - fall_amount * 0.025).clamp(-0.14, 0.14);
        rx += (pitch_angle * 0.55).clamp(-22.0, 22.0);
        ry += (-yaw_angle * 0.65).clamp(-24.0, 24.0);
        rz += (yaw_angle * 0.35).clamp(-16.0, 16.0);

        // 4. Custom swing easing curves (item_pose.lua lines 190-222)
        let swing_eased = if kind == ItemKind::Pickaxe {
            ease_custom(swing_raw)
        } else {
            ease_custom_sec(swing_raw)
        };
        let swing_rot_base = if swing_eased < 0.70016 {
            (swing_eased.clamp(0.0, 0.308) * 5.1).sin()
        } else {
            (swing_eased.clamp(0.70016, 1.0) * 5.1 - 2.0).sin()
        };
        let swing_rot = swing_rot_base * swing_rot_base * swing_rot_base;
        let swing_sword_tilt = if swing_eased < 0.65245 {
            (swing_eased.clamp(0.0, 0.16675) * PI * 3.0).sin()
        } else {
            (swing_eased.clamp(0.65245, 1.0) * 4.4 - 1.3).sin()
        };
        let swing_hit = (swing_eased.clamp(0.16561, 0.49422) * 4.78 * 2.0 + 4.7).sin();
        let swing_overall = (swing_eased * PI).sin();

        // 5. Item-category poses & swing animations
        match kind {
            ItemKind::Sword => {
                // Slightly lower, more natural combat grip
                ty -= 0.04;
                tz -= 0.03;
                rx -= 6.0;
                if swing_raw > 0.001 {
                    let s = ease_in_out_back(swing_overall);
                    ty -= 0.09 * s;
                    tz -= 0.08 * swing_rot;
                    rx -= 48.0 * s + 12.0 * swing_sword_tilt;
                    ry += 14.0 * swing_overall * side_sign;
                    rz -= 18.0 * swing_overall * side_sign;
                }
            }
            ItemKind::Pickaxe | ItemKind::Axe | ItemKind::Hoe | ItemKind::Mace => {
                ty -= 0.02;
                if swing_raw > 0.001 {
                    tz -= 0.09 * swing_rot;
                    ty += 0.16 * swing_overall - 0.05 * swing_rot + 0.04 * swing_hit;
                    rx -= 10.0 * swing_rot + 15.0 * swing_hit + 38.0 * swing_sword_tilt;
                }
            }
            ItemKind::Shovel => {
                if swing_raw > 0.001 {
                    ty += 0.10 * swing_sword_tilt;
                    tz += 0.05 * swing_sword_tilt;
                    rx += 10.0 * swing_sword_tilt - 28.0 * swing_overall + 18.0 * swing_rot;
                }
            }
            ItemKind::Trident | ItemKind::Spear => {
                if swing_raw > 0.001 {
                    tz -= 0.10 * swing_rot;
                    ty += 0.14 * swing_overall - 0.05 * swing_rot;
                    rx -= 40.0 * swing_overall + 12.0 * swing_hit;
                }
                if use_anim > 0.01 {
                    let u = ease_in_out_sine(use_anim);
                    tz += 0.10 * u;
                    ty += 0.06 * u;
                    rx -= 28.0 * u;
                }
            }
            ItemKind::Bow => {
                tx -= 0.055 * side_sign;
                if use_anim > 0.01 {
                    let wobble = (idle_time * 18.0).sin() * 0.004 * use_anim;
                    tx += wobble;
                    ty += wobble * 0.5;
                    tz += 0.04 * use_anim;
                }
            }
            ItemKind::Crossbow => {
                if use_anim > 0.01 {
                    tz += 0.05 * use_anim;
                    rx -= 6.0 * use_anim;
                }
            }
            ItemKind::Shield => {
                if use_anim > 0.01 {
                    let u = ease_in_out_sine(use_anim);
                    tx -= 0.06 * u * side_sign;
                    ty += 0.04 * u;
                    ry += 12.0 * u * side_sign;
                }
            }
            ItemKind::FoodOrDrink => {
                if use_anim > 0.01 {
                    let chew = (use_remaining as f32 * 0.8 + event.render.partial * 0.8).sin();
                    let u = ease_in_out_sine(use_anim);
                    tx -= 0.08 * u * side_sign;
                    ty += (0.04 + chew * 0.018) * u;
                    tz += 0.06 * u;
                    rx += (10.0 + chew * 5.0) * u;
                    rz += -12.0 * u * side_sign;
                }
            }
            ItemKind::TorchOrLantern => {
                ty -= 0.05;
                tz -= 0.03;
                rx -= 12.0;
                rz += 4.0 * side_sign;
            }
            ItemKind::Bucket => {
                ty -= 0.03;
                rx -= 5.0;
            }
            ItemKind::ThrowableOrSmall => {
                let sw_curve = ((switch_anim.clamp(0.09723, 0.60632) * 3.24 * 1.65 - 0.1).sin())
                    .clamp(0.0, 1.0);
                let sw_eased = ease_in_out_back(sw_curve);
                rx += 10.0 * sw_eased;
                rz += 6.0 * sw_eased * side_sign;
            }
            ItemKind::Bed => {
                tz += 0.12;
                ry += 25.0 * side_sign;
            }
            // Unreachable in normal flow: maps return to vanilla rendering above.
            ItemKind::Map => {}
            ItemKind::Empty | ItemKind::Generic => {
                if swing_raw > 0.001 {
                    tz -= 0.05 * swing_rot;
                    ty -= 0.04 * swing_rot;
                    rx -= 10.0 * swing_rot + 20.0 * swing_hit;
                }
            }
        }

        // Apply final pose to vanilla/resourcepack item geometry
        event.render.translation[0] += tx.clamp(-1.5, 1.5);
        event.render.translation[1] += ty.clamp(-1.5, 1.5);
        event.render.translation[2] += tz.clamp(-1.5, 1.5);
        event.render.rotation[0] += rx.clamp(-180.0, 180.0);
        event.render.rotation[1] += ry.clamp(-180.0, 180.0);
        event.render.rotation[2] += rz.clamp(-180.0, 180.0);

        // 6. Draw 3D first-person player arm holding the item (Hold My Items signature feature)
        if kind != ItemKind::Empty && kind != ItemKind::Shield {
            let arm_part = if is_main {
                &state.arm_right
            } else {
                &state.arm_left
            };
            event.draw.push();
            // Position forearm in hands' view space (+x right, +y up, -z forward)
            let equip_drop = -equip_raw * 0.45;
            event.draw.translate([
                0.44 * side_sign + tx * 0.75,
                -0.58 + equip_drop + ty * 0.75,
                -0.60 + tz * 0.65,
            ]);
            event.draw.rotate_about([1.0, 0.0, 0.0], 68.0 + rx * 0.45, [0.0; 3]);
            event
                .draw
                .rotate_about([0.0, 1.0, 0.0], -20.0 * side_sign + ry * 0.45, [0.0; 3]);
            event
                .draw
                .rotate_about([0.0, 0.0, 1.0], 16.0 * side_sign + rz * 0.45, [0.0; 3]);
            event.draw.scale([0.85, 0.85, 0.85]);
            let _ = event
                .draw
                .model_part(arm_part, "entity/player/wide/steve", 0xFFFFFF);
            event.draw.pop();
        }

        Flow::Continue
    }
}
