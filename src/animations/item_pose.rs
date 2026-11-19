//! Per-item pose, spring physics, custom sounds, and first-person particle spawning
//! ported from `item_pose.lua` (HMI 5.1.1).

use crate::animations::FrameContext;
use crate::easings;
use crate::item_info::UseAction;
use crate::math::{self, PoseStack, PI};
use crate::particles::{Particle, ParticleLifecycle, ParticleRenderType, RenderSpace};
use crate::state::HmiState;

pub fn apply_item_pose(mat: &mut PoseStack, ctx: &FrameContext<'_>, state: &mut HmiState) {
    let l = if ctx.is_right_arm { 1.0 } else { -1.0 };
    let dt = ctx.delta_time.clamp(0.0, 0.05);
    let dt30 = dt * 30.0;
    let clock = state.registry.get_f64("a", 0.0);

    // 1. Sword slash sound trigger + blade flourish
    if ctx.item.is_sword && !ctx.block_breaking {
        let prev_swing_toggle_key = if ctx.main_hand {
            "lastSwordToggleM"
        } else {
            "lastSwordToggleO"
        };
        let cur_toggle = if ctx.main_hand {
            ctx.swing_main_toggle
        } else {
            ctx.swing_off_toggle
        };
        let prev_toggle = state.registry.get_bool(prev_swing_toggle_key, cur_toggle);
        if cur_toggle != prev_toggle {
            state.registry.set_bool(prev_swing_toggle_key, cur_toggle);
            state.queue_sound("minecraft:custom.slashy", 0.55, 1.0);
        }

        // Slight angled combat stance for swords
        mat.rotate_x(-6.0);
        mat.rotate_z(4.0 * l);
        let swing = ctx.swing_progress.clamp(0.0, 1.0);
        if swing > 0.001 {
            let s = easings::cubic_ease((swing * PI).sin());
            let alt = if cur_toggle { 1.0 } else { -1.0 };
            mat.rotate_x(-24.0 * s);
            mat.rotate_y(28.0 * s * l * alt);
            mat.move_z(-0.08 * s);
        }
    }

    // 2. Torches & Lanterns: gentle sway + flame & glow first-person particles
    if ctx.item.is_torch || ctx.item.is_lantern {
        let sway = (clock * 2.2).sin();
        if ctx.item.is_lantern {
            // Lantern pendulum inertia
            let vel_sway = (ctx.player.velocity[0] + ctx.player.velocity[2]).clamp(-0.5, 0.5);
            mat.rotate_z_pivot((sway * 4.5 + vel_sway * 18.0) * l, 0.0, 0.25, 0.0);
            mat.rotate_x_pivot(sway * 2.5, 0.0, 0.25, 0.0);
        } else {
            mat.rotate_z(sway * 2.0 * l);
        }

        if state.settings.particles {
            maybe_spawn_flame_particles(ctx, state);
        }
    }

    // 3. Pink petals, wildflowers, leaf litter drifting particles
    if ctx.item.is_flower_or_petal && state.settings.particles {
        maybe_spawn_nature_particles(ctx, state);
    }

    // 4. Bow draw spring (`bowPullAngle` / `bowPullSpeed`)
    if ctx.item.use_action == UseAction::Bow {
        let bow_key = if ctx.main_hand { "bowCount" } else { "bowCountO" };
        let bow = state.registry.get_f64(bow_key, 0.0);
        if bow > 0.01 {
            let pull_angle_key = if ctx.main_hand {
                "bowPullAngle"
            } else {
                "bowPullAngleO"
            };
            let pull_speed_key = if ctx.main_hand {
                "bowPullSpeed"
            } else {
                "bowPullSpeedO"
            };
            let mut angle = state.registry.get_f64(pull_angle_key, 0.0);
            let mut speed = state.registry.get_f64(pull_speed_key, 0.0);
            let target = easings::ease_out_back(bow);
            speed += (target - angle) * 0.35 * dt30;
            speed *= 0.78_f64.powf(dt30);
            angle += speed * dt30;
            state.registry.set_f64(pull_angle_key, angle);
            state.registry.set_f64(pull_speed_key, speed);

            mat.scale(1.0, 1.0, 1.0 + angle.clamp(0.0, 1.5) * 0.125);
        }
    }

    // 5. Food & Drink secondary wobble
    if matches!(ctx.item.use_action, UseAction::Eat | UseAction::Drink)
        && ctx.player.using_item
        && ctx.player.active_hand == Some(ctx.hand)
    {
        let wobble = (clock * 20.0).sin();
        mat.rotate_z(wobble * 6.0 * l);
        mat.move_y(wobble.abs() * 0.02);
    }

    // 6. Buckets & Boats subtle weight tilt
    if ctx.item.is_bucket {
        let vel_y = ctx.player.velocity[1].clamp(-0.5, 0.5);
        mat.rotate_x(vel_y * 12.0);
        mat.rotate_z(-4.0 * l);
    } else if ctx.item.is_boat {
        let bob = (clock * 1.8).sin();
        mat.rotate_z(bob * 3.0 * l);
        mat.move_y(bob * 0.015);
    }

    // 7. Bundles & Throwables
    if ctx.item.is_bundle {
        let b = (clock * 2.4).sin();
        mat.rotate_z_pivot(b * 3.5 * l, 0.0, 0.2, 0.0);
    } else if ctx.item.is_throwable {
        let sway = (clock * 3.0).sin();
        mat.move_y(sway * 0.012);
        mat.rotate_y(sway * 5.0 * l);
    }

    // 8. Shield resting pose
    if ctx.item.use_action == UseAction::Block {
        mat.move_x(-0.04 * l);
        mat.rotate_y(6.0 * l);
    }

    // 9. Brush oscillation
    if ctx.item.use_action == UseAction::Brush
        && ctx.player.using_item
        && ctx.player.active_hand == Some(ctx.hand)
    {
        let osc = (clock * 16.0).sin();
        mat.rotate_z(osc * 18.0 * l);
        mat.move_x(osc * 0.05 * l);
    }

    // 10. Compass / Clock subtle inspection tilt
    if ctx.item.is_compass_or_clock {
        mat.rotate_x(12.0);
        mat.rotate_y(-10.0 * l);
    }

    // 11. Block-in-hand framing when rendered as 3D block
    if ctx.item.should_render_as_block(state) {
        mat.scale(0.92, 0.92, 0.92);
        if state
            .apply_block_rotation
            .get(&ctx.item.id)
            .copied()
            .unwrap_or(true)
        {
            mat.rotate_y(12.0 * l);
        }
    }

    // 12. Standard non-tool item translate offset (`I.shouldTranslateItem`)
    if ctx.item.should_translate_item()
        && !ctx.item.is_block()
        && !ctx.item.is_map
        && state
            .translate_item
            .get(&ctx.item.id)
            .copied()
            .unwrap_or(true)
    {
        mat.move_y(0.015);
    }

    let _ = math::clamp(0.0, 0.0, 1.0);
}

fn maybe_spawn_flame_particles(ctx: &FrameContext<'_>, state: &mut HmiState) {
    let key = if ctx.main_hand {
        "lastFlameTickM"
    } else {
        "lastFlameTickO"
    };
    let last_tick = state.registry.get_f64(key, -10.0) as i32;
    if (ctx.player.age - last_tick).abs() < 6 {
        return;
    }
    state.registry.set_f64(key, ctx.player.age as f64);

    let (glow_sprite, is_soul) = if ctx.item.id.contains("soul") {
        ("minecraft:particle/blue_glow", true)
    } else if ctx.item.id.contains("redstone") {
        ("minecraft:particle/red_glow", false)
    } else if ctx.item.id.contains("copper") {
        ("minecraft:particle/copper_glow", false)
    } else {
        ("minecraft:particle/orange_glow", false)
    };

    let l = if ctx.is_right_arm { 1.0 } else { -1.0 };
    let y_off = if ctx.item.is_lantern { 0.05 } else { 0.24 };

    // Soft additive glow aura
    state.particles.push(Particle::new(
        false,
        [0.02 * l, y_off, -0.02],
        [0.0, 0.004, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.5 * l],
        if is_soul { 0.36 } else { 0.32 },
        glow_sprite,
        RenderSpace::Item,
        ctx.hand,
        ParticleLifecycle::Opacity,
        ParticleRenderType::Additive,
        0.9,
        175.0,
    ));

    // Animated keyframe flame for regular/soul torches
    if ctx.item.is_torch && !ctx.item.id.contains("redstone") {
        let frames: Vec<String> = (0..=14)
            .map(|i| format!("minecraft:particle/flame_{i}"))
            .collect();
        let has_keyframe = state
            .particles
            .iter()
            .any(|p| p.lifecycle == ParticleLifecycle::Keyframe && p.hand == ctx.hand);
        if !has_keyframe {
            state.particles.push(
                Particle::new(
                    false,
                    [0.0, y_off + 0.04, 0.0],
                    [0.0, 0.002, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    0.26,
                    "minecraft:particle/flame_0",
                    RenderSpace::Item,
                    ctx.hand,
                    ParticleLifecycle::Keyframe,
                    ParticleRenderType::CutoutLit,
                    2.5,
                    255.0,
                )
                .with_keyframes(frames),
            );
        }
    }
}

fn maybe_spawn_nature_particles(ctx: &FrameContext<'_>, state: &mut HmiState) {
    let key = if ctx.main_hand {
        "lastPetalTickM"
    } else {
        "lastPetalTickO"
    };
    let last_tick = state.registry.get_f64(key, -20.0) as i32;
    if (ctx.player.age - last_tick).abs() < 12 {
        return;
    }
    state.registry.set_f64(key, ctx.player.age as f64);

    let idx = (ctx.player.age.unsigned_abs() % 4) + 1;
    let sprite = if ctx.item.is_of("minecraft:pink_petals") {
        format!("minecraft:particle/pink_petals_{idx}")
    } else if ctx.item.is_of("minecraft:wildflowers") {
        format!("minecraft:particle/wild_flowers_{idx}")
    } else if ctx.item.is_of("minecraft:leaf_litter") {
        format!("minecraft:particle/leaf_litter_{idx}")
    } else {
        return;
    };

    let l = if ctx.is_right_arm { 1.0 } else { -1.0 };
    let drift = ((ctx.player.age as f64) * 1.7).sin() * 0.008;
    state.particles.push(Particle::new(
        true,
        [0.04 * l, 0.12, -0.02],
        [drift, 0.012, -0.004],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 2.5 * l],
        0.18,
        sprite,
        RenderSpace::Item,
        ctx.hand,
        ParticleLifecycle::Scale,
        ParticleRenderType::Cutout,
        1.6,
        255.0,
    ));
}
