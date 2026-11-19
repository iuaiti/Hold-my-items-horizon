//! Scene-level hand & camera animation ported from `hand_pose.lua` (HMI 5.1.1).
//!
//! Handles breathing, walking/sprinting bob, camera pitch/yaw sway, fall/jump
//! inertia, sneaking, swimming, crawling, climbing, equip/switch transitions,
//! melee/mining swings, bow/crossbow/trident/spear/shield/eat/drink/brush/spyglass
//! use poses, inspect keybind, user X/Y/Z offsets, and synchronized camera shake.

use crate::animations::FrameContext;
use crate::easings;
use crate::item_info::UseAction;
use crate::math::{self, PoseStack, PI};
use crate::state::HmiState;

pub fn apply_scene_pose(mat: &mut PoseStack, ctx: &FrameContext<'_>, state: &mut HmiState) {
    let l = if ctx.is_right_arm { 1.0 } else { -1.0 };
    let dt = ctx.delta_time.clamp(0.0, 0.05);
    let dt30 = dt * 30.0;

    // Advance global animation clock once on main hand pass
    let mut clock = state.registry.get_f64("a", 0.0);
    if ctx.main_hand {
        clock = (clock + dt * 2.5) % (PI * 200.0);
        state.registry.set_f64("a", clock);
    }

    // 1. User configured X / Y / Z offsets (from Options > Mods settings, replacing respackopts)
    mat.translate(
        state.settings.x_offset * l,
        state.settings.y_offset,
        state.settings.z_offset,
    );

    // 2. Smooth hand-switch / equip counters (`mainHandSwitch` / `offHandSwitch`)
    let switch_key = if ctx.main_hand {
        "mainHandSwitch"
    } else {
        "offHandSwitch"
    };
    let mut switch_val = state.registry.get_f64(switch_key, 1.0);
    let switch_triggered = if ctx.main_hand {
        ctx.main_hand_switch_event
    } else {
        ctx.off_hand_switch_event
    };
    if switch_triggered {
        switch_val = 0.0;
    } else {
        switch_val = (switch_val + dt * 3.8).min(1.0);
    }
    state.registry.set_f64(switch_key, switch_val);

    let switch_ease = easings::ease_out_back(switch_val.clamp(0.0, 1.0));
    let switch_drop = (1.0 - switch_ease) * 0.45;
    mat.move_y(-switch_drop);
    mat.rotate_x(-22.0 * (1.0 - switch_val));

    // 3. Breathing / idle micro-motion
    let breath = (clock * 1.4).sin();
    let breath_cos = (clock * 0.7).cos();
    mat.move_y(breath * 0.008);
    mat.move_x(breath_cos * 0.004 * l);
    mat.rotate_z(breath * 0.6 * l);

    // 4. Walking / sprinting bob & smoother (`walk`, `walkSmoother`)
    let horiz_speed = ctx.player.horizontal_speed();
    let moving = ctx.player.on_ground && horiz_speed > 0.015;
    let mut walk = state.registry.get_f64("walk", 0.0);
    let mut walk_smoother = state.registry.get_f64("walkSmoother", 0.0);
    if ctx.main_hand {
        let target = if moving {
            (horiz_speed * 5.0).clamp(0.0, 1.35)
        } else {
            0.0
        };
        walk_smoother = math::lerp((dt * 10.0).min(1.0), walk_smoother, target);
        let speed_mul = if ctx.player.sprinting { 11.5 } else { 8.0 };
        walk = (walk + dt * speed_mul * walk_smoother.max(0.25)) % (PI * 200.0);
        state.registry.set_f64("walk", walk);
        state.registry.set_f64("walkSmoother", walk_smoother);
    }

    if walk_smoother > 0.005 {
        let bob_x = walk.cos() * 0.032 * walk_smoother * l;
        let bob_y = (walk * 2.0).sin().abs() * 0.028 * walk_smoother;
        let bob_z = walk.sin() * 0.018 * walk_smoother;
        mat.translate(bob_x, -bob_y, bob_z);
        mat.rotate_z(walk.cos() * 2.2 * walk_smoother * l);
        mat.rotate_x((walk * 2.0).sin() * 1.5 * walk_smoother);
        if ctx.player.sprinting && !ctx.player.using_item {
            mat.rotate_y(-8.0 * walk_smoother * l);
            mat.move_z(0.04 * walk_smoother);
        }
    }

    // 5. Camera pitch/yaw inertia sway (`yawAngle`, `pitchAngle`)
    let prev_yaw = state.registry.get_f64("prevYaw", ctx.player.yaw);
    let prev_pitch = state.registry.get_f64("prevPitch", ctx.player.pitch);
    let mut dyaw = ctx.player.yaw - prev_yaw;
    while dyaw > 180.0 {
        dyaw -= 360.0;
    }
    while dyaw < -180.0 {
        dyaw += 360.0;
    }
    let dpitch = ctx.player.pitch - prev_pitch;

    let yaw_key = if ctx.main_hand { "yawAngle" } else { "yawAngleO" };
    let pitch_key = if ctx.main_hand { "pitchAngle" } else { "pitchAngleO" };
    let mut yaw_angle = state.registry.get_f64(yaw_key, 0.0);
    let mut pitch_angle = state.registry.get_f64(pitch_key, 0.0);
    yaw_angle = math::lerp((dt * 12.0).min(1.0), yaw_angle, dyaw.clamp(-18.0, 18.0));
    pitch_angle = math::lerp((dt * 12.0).min(1.0), pitch_angle, dpitch.clamp(-15.0, 15.0));
    state.registry.set_f64(yaw_key, yaw_angle);
    state.registry.set_f64(pitch_key, pitch_angle);
    if ctx.main_hand {
        state.registry.set_f64("prevYaw", ctx.player.yaw);
        state.registry.set_f64("prevPitch", ctx.player.pitch);
    }

    mat.move_x(-yaw_angle * 0.0045);
    mat.move_y(pitch_angle * 0.004);
    mat.rotate_y(-yaw_angle * 0.55);
    mat.rotate_x(-pitch_angle * 0.45);
    mat.rotate_z(-yaw_angle * 0.25);

    // 6. Vertical velocity / fall inertia (`fall`)
    let mut fall = state.registry.get_f64("fall", 0.0);
    if ctx.main_hand {
        let target_fall = if ctx.player.on_ground {
            0.0
        } else {
            (-ctx.player.velocity[1]).clamp(-0.6, 0.8)
        };
        fall = math::lerp((dt * 9.0).min(1.0), fall, target_fall);
        state.registry.set_f64("fall", fall);
    }
    if fall.abs() > 0.005 {
        mat.move_y(fall * 0.085);
        mat.rotate_x(fall * 8.0);
    }

    // 7. Sneaking, swimming, crawling, climbing poses
    let mut sneak_t = state.registry.get_f64("sneakAnim", 0.0);
    let mut swim_t = state.registry.get_f64("swimAnim", 0.0);
    let mut crawl_t = state.registry.get_f64("crawlAnimation", 0.0);
    let mut climb_t = state.registry.get_f64("climbAnimation", 0.0);
    if ctx.main_hand {
        sneak_t = math::lerp((dt * 10.0).min(1.0), sneak_t, if ctx.player.sneaking { 1.0 } else { 0.0 });
        swim_t = math::lerp((dt * 7.0).min(1.0), swim_t, if ctx.player.swimming { 1.0 } else { 0.0 });
        crawl_t = math::lerp((dt * 7.0).min(1.0), crawl_t, if ctx.player.crawling { 1.0 } else { 0.0 });
        climb_t = math::lerp((dt * 8.0).min(1.0), climb_t, if ctx.player.climbing { 1.0 } else { 0.0 });
        state.registry.set_f64("sneakAnim", sneak_t);
        state.registry.set_f64("swimAnim", swim_t);
        state.registry.set_f64("crawlAnimation", crawl_t);
        state.registry.set_f64("climbAnimation", climb_t);
    }

    if sneak_t > 0.01 {
        mat.translate(-0.04 * sneak_t * l, -0.055 * sneak_t, 0.03 * sneak_t);
        mat.rotate_z(5.0 * sneak_t * l);
    }
    if swim_t > 0.01 {
        let stroke = (clock * 3.0 + if ctx.main_hand { 0.0 } else { PI }).sin();
        mat.translate(0.06 * stroke * swim_t * l, -0.08 * swim_t + 0.05 * stroke.abs() * swim_t, -0.08 * swim_t);
        mat.rotate_x(-22.0 * swim_t + 12.0 * stroke * swim_t);
        mat.rotate_z(15.0 * swim_t * l);
    }
    if crawl_t > 0.01 {
        let crawl_wave = (clock * 2.5 + if ctx.main_hand { 0.0 } else { PI }).sin();
        mat.translate(0.05 * crawl_t * l, -0.10 * crawl_t, -0.10 * crawl_t + 0.06 * crawl_wave * crawl_t);
        mat.rotate_x(-18.0 * crawl_t);
    }
    if climb_t > 0.01 {
        let climb_wave = (ctx.player.pos[1] * 4.5 + if ctx.main_hand { 0.0 } else { PI }).sin();
        mat.move_y(climb_wave * 0.12 * climb_t);
        mat.rotate_x(climb_wave * 14.0 * climb_t);
    }

    // 8. Active item use animations (Bow, Crossbow, Trident, Spear, Shield, Eat, Drink, Brush, Spyglass)
    let is_using_this_hand =
        ctx.player.using_item && ctx.player.active_hand == Some(ctx.hand);
    let mut cam_pitch = 0.0_f32;
    let mut cam_yaw = 0.0_f32;
    let mut cam_roll = 0.0_f32;
    let mut cam_pos = [0.0_f32; 3];

    match ctx.item.use_action {
        UseAction::Bow => {
            let key = if ctx.main_hand { "bowCount" } else { "bowCountO" };
            let mut bow = state.registry.get_f64(key, 0.0);
            bow = if is_using_this_hand {
                (bow + dt * 1.4).min(1.0)
            } else {
                (bow - dt * 5.0).max(0.0)
            };
            state.registry.set_f64(key, bow);
            if bow > 0.005 {
                let eased = easings::ease_out_quart(bow);
                let wiggle = if bow > 0.85 {
                    (clock * 28.0).sin() * 0.004 * (bow - 0.85) * 6.0
                } else {
                    0.0
                };
                mat.translate(-0.14 * eased * l + wiggle, 0.05 * eased + wiggle, 0.16 * eased);
                mat.rotate_y(18.0 * eased * l);
                mat.rotate_z(-10.0 * eased * l);
                cam_pos[2] += (0.02 * eased) as f32;
            }
        }
        UseAction::Crossbow => {
            let key = if ctx.main_hand { "crossBowM" } else { "crossBowO" };
            let mut cb = state.registry.get_f64(key, 0.0);
            let target = if is_using_this_hand || ctx.item.charged_crossbow {
                1.0
            } else {
                0.0
            };
            cb = math::lerp((dt * 8.0).min(1.0), cb, target);
            state.registry.set_f64(key, cb);
            if cb > 0.005 {
                let eased = easings::ease_in_out_sine(cb);
                mat.translate(-0.12 * eased * l, -0.02 * eased, 0.08 * eased);
                mat.rotate_y(12.0 * eased * l);
            }
        }
        UseAction::Trident => {
            let key = if ctx.main_hand { "tridentM" } else { "tridentO" };
            let mut tr = state.registry.get_f64(key, 0.0);
            tr = if is_using_this_hand {
                (tr + dt * 2.5).min(1.0)
            } else {
                (tr - dt * 6.0).max(0.0)
            };
            state.registry.set_f64(key, tr);
            if tr > 0.005 {
                let eased = easings::ease_in_out_back(tr);
                mat.translate(0.08 * eased * l, 0.16 * eased, 0.18 * eased);
                mat.rotate_x(38.0 * eased);
                mat.rotate_z(-12.0 * eased * l);
                cam_pitch -= (1.2 * eased) as f32;
            }
            if ctx.player.using_riptide {
                mat.rotate_z((clock * 45.0).sin() * 25.0 * l);
            }
        }
        UseAction::Spear => {
            let key = if ctx.main_hand { "spearCounterM" } else { "spearCounterO" };
            let mut sp = state.registry.get_f64(key, 0.0);
            sp = if is_using_this_hand {
                (sp + dt * 2.2).min(1.0)
            } else {
                (sp - dt * 5.5).max(0.0)
            };
            state.registry.set_f64(key, sp);
            if sp > 0.005 {
                let eased = easings::ease_out_back(sp);
                mat.translate(-0.08 * eased * l, 0.06 * eased, -0.22 * eased);
                mat.rotate_x(-28.0 * eased);
                if ctx.item.spear_data.can_dismount {
                    mat.move_z(-0.08 * (clock * 18.0).sin());
                }
            }
        }
        UseAction::Block => {
            let key = if ctx.main_hand { "shieldM" } else { "shieldO" };
            let mut sh = state.registry.get_f64(key, 0.0);
            sh = if is_using_this_hand {
                (sh + dt * 8.0).min(1.0)
            } else {
                (sh - dt * 7.0).max(0.0)
            };
            state.registry.set_f64(key, sh);
            if sh > 0.005 {
                let eased = easings::ease_in_out_sine(sh);
                mat.translate(-0.22 * eased * l, 0.12 * eased, 0.10 * eased);
                mat.rotate_y(28.0 * eased * l);
                mat.rotate_z(-14.0 * eased * l);
                cam_roll += (1.1 * eased * l) as f32;
            }
        }
        UseAction::Eat => {
            let key = if ctx.main_hand { "foodCount" } else { "foodCountO" };
            let mut fc = state.registry.get_f64(key, 0.0);
            fc = if is_using_this_hand {
                (fc + dt * 5.0).min(1.0)
            } else {
                (fc - dt * 6.0).max(0.0)
            };
            state.registry.set_f64(key, fc);
            if fc > 0.005 {
                let chew = (clock * 16.0).sin().abs();
                let eased = easings::ease_in_out_sine(fc);
                mat.translate(-0.24 * eased * l, 0.10 * eased + chew * 0.035 * eased, 0.12 * eased);
                mat.rotate_x(18.0 * eased + chew * 8.0 * eased);
                mat.rotate_y(32.0 * eased * l);
                cam_pitch += (chew * 0.45 * eased) as f32;
            }
        }
        UseAction::Drink => {
            let key = if ctx.main_hand { "drinkCount" } else { "drinkCountO" };
            let mut dc = state.registry.get_f64(key, 0.0);
            dc = if is_using_this_hand {
                (dc + dt * 4.5).min(1.0)
            } else {
                (dc - dt * 5.5).max(0.0)
            };
            state.registry.set_f64(key, dc);
            if dc > 0.005 {
                let gulp = (clock * 12.0).sin();
                let eased = easings::ease_in_out_sine(dc);
                mat.translate(-0.26 * eased * l, 0.14 * eased + gulp * 0.02 * eased, 0.14 * eased);
                mat.rotate_x(35.0 * eased + gulp * 5.0 * eased);
                mat.rotate_z(-22.0 * eased * l);
                cam_pitch -= (0.6 * eased) as f32;
            }
        }
        UseAction::Brush => {
            let key = if ctx.main_hand { "brushCounter" } else { "brushCounterO" };
            let mut br = state.registry.get_f64(key, 0.0);
            br = if is_using_this_hand {
                (br + dt * 5.0).min(1.0)
            } else {
                (br - dt * 6.0).max(0.0)
            };
            state.registry.set_f64(key, br);
            if br > 0.005 {
                let sweep = (clock * 14.0).sin();
                mat.translate(sweep * 0.12 * br * l, -0.02 * br, -0.08 * br);
                mat.rotate_z(sweep * 22.0 * br * l);
            }
        }
        UseAction::Spyglass => {
            if is_using_this_hand {
                mat.translate(-0.28 * l, 0.18, 0.15);
                mat.rotate_x(12.0);
            }
        }
        UseAction::TootHorn => {
            if is_using_this_hand {
                mat.translate(-0.22 * l, 0.14, 0.10);
                mat.rotate_x(25.0);
                mat.rotate_y(25.0 * l);
            }
        }
        UseAction::None | UseAction::Bundle => {}
    }

    // 9. Swing / attack motion in scene space
    let swing = ctx.swing_progress.clamp(0.0, 1.0);
    if swing > 0.001 {
        let s = (swing * PI).sin();
        let s_ease = easings::ease_in_out_sine(s);
        let alt = if (ctx.main_hand && ctx.swing_main_toggle)
            || (!ctx.main_hand && ctx.swing_off_toggle)
        {
            1.0
        } else {
            -1.0
        };

        if ctx.item.is_sword && !ctx.block_breaking {
            // Alternating horizontal/diagonal sword slash
            mat.translate(-0.22 * s_ease * l * alt, -0.08 * s_ease, -0.14 * s_ease);
            mat.rotate_y(42.0 * s_ease * l * alt);
            mat.rotate_x(-28.0 * s_ease);
            mat.rotate_z(-18.0 * s_ease * l * alt);
            cam_yaw += (1.35 * s_ease * l * alt) as f32;
            cam_roll += (0.85 * s_ease * l * alt) as f32;
        } else if ctx.item.is_axe || ctx.item.is_mace {
            // Heavy overhead chop / slam
            let chop = easings::ease_in_out_expo(swing);
            let arc = (chop * PI).sin();
            mat.translate(-0.10 * arc * l, 0.10 * (1.0 - swing) * arc - 0.18 * arc, -0.16 * arc);
            mat.rotate_x(-58.0 * arc);
            mat.rotate_z(-12.0 * arc * l);
            cam_pitch += (1.8 * arc) as f32;
        } else if ctx.block_breaking || ctx.item.is_pickaxe || ctx.item.is_shovel || ctx.item.is_hoe {
            // Crisp tool mining strike
            mat.translate(-0.08 * s * l, -0.12 * s, -0.12 * s);
            mat.rotate_x(-44.0 * s);
            mat.rotate_z(8.0 * s * l);
            cam_pitch += (0.75 * s) as f32;
        } else {
            // General hand / item swing
            mat.translate(-0.12 * s * l, -0.08 * s, -0.10 * s);
            mat.rotate_x(-32.0 * s);
            mat.rotate_y(16.0 * s * l);
        }
    }

    // 10. Inspect keybind animation (J by default)
    let mut inspect_t = state.registry.get_f64("inspectAnim", 0.0);
    if ctx.main_hand {
        let target = if state.inspect_held && !ctx.item.empty {
            1.0
        } else {
            0.0
        };
        inspect_t = math::lerp((dt30 * 0.2).min(1.0), inspect_t, target);
        state.registry.set_f64("inspectAnim", inspect_t);
    }
    if ctx.main_hand && inspect_t > 0.005 {
        let eased = easings::ease_in_out_back(inspect_t.clamp(0.0, 1.0));
        let spin = (clock * 1.5).sin();
        mat.translate(-0.24 * eased * l, 0.12 * eased, 0.14 * eased);
        mat.rotate_y((38.0 + spin * 24.0) * eased * l);
        mat.rotate_x((12.0 + spin * 6.0) * eased);
        mat.rotate_z(-15.0 * eased * l);
    }

    // 11. Subtle baseline offset matching HeldItemRendererMixin.scenePoseMain
    if !ctx.item.empty {
        mat.translate(0.0, -0.04, 0.02);
    }

    // 12. Commit synchronized camera shake (only from main hand to avoid doubling)
    if ctx.main_hand {
        if state.settings.camera_shake {
            state.camera.pitch = cam_pitch;
            state.camera.yaw = cam_yaw;
            state.camera.roll = cam_roll;
            state.camera.pos = cam_pos;
        } else {
            state.camera = Default::default();
        }
    }
}
