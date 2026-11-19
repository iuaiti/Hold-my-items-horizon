//! Per-element 3D item model animation ported from `item_model.lua` (HMI 5.1.1).
//!
//! Populates `state.model_animator` for animated 3D item models bundled in `pack_test`:
//! - Axolotl, Cod, Salmon, Pufferfish, Tropical Fish, Tadpole buckets (head/body/fin/liquid motion)
//! - Water, Lava, Milk, Powder Snow buckets (sloshing liquid surface)
//! - Chest Boats & Rafts (bouncing chest lid on walk/fall)

use crate::animations::FrameContext;
use crate::easings;
use crate::math::{self, PoseStack};
use crate::state::HmiState;

pub fn apply_item_model_pose(
    mat: &mut PoseStack,
    ctx: &FrameContext<'_>,
    state: &mut HmiState,
) {
    let l = if ctx.is_right_arm { 1.0 } else { -1.0 };
    let clock = state.registry.get_f64("a", 0.0);
    let walk = state.registry.get_f64("walk", 0.0);
    let walk_smoother = state.registry.get_f64("walkSmoother", 0.0);
    let fall = state.registry.get_f64("fall", 0.0);

    let wf = (walk_smoother + fall.abs()).clamp(0.0, 1.5);

    if ctx.item.is_of("minecraft:axolotl_bucket")
        || ctx.item.is_of("minecraft:cod_bucket")
        || ctx.item.is_of("minecraft:salmon_bucket")
        || ctx.item.is_of("minecraft:pufferfish_bucket")
        || ctx.item.is_of("minecraft:tropical_fish_bucket")
        || ctx.item.is_of("minecraft:tadpole_bucket")
    {
        let swim_wave = (clock * 6.0).sin();
        let wiggle = (clock * 3.5).sin() * wf;

        // Head elements (0..=11)
        state.model_animator.rotate_y(
            0,
            11,
            (swim_wave * 10.0 + wiggle * 8.0) as f32,
            [0.5, 0.6, 0.5],
        );
        // Body / tail elements (12..=19)
        state.model_animator.rotate_y(
            12,
            19,
            (-swim_wave * 14.0) as f32,
            [0.5, 0.6, 0.5],
        );
        // Fins / liquid surface (20..=23)
        state.model_animator.rotate_z(
            20,
            23,
            (wiggle * 6.0 * l) as f32,
            [0.5, 0.5, 0.5],
        );

        // Subtle whole-bucket slosh feedback
        mat.rotate_z(wiggle * 2.5 * l);
    } else if ctx.item.is_bucket {
        let slosh = (walk.sin() * walk_smoother * 4.0 + fall * 6.0).clamp(-10.0, 10.0);
        state
            .model_animator
            .rotate_z(0, 8, (slosh * l) as f32, [0.5, 0.5, 0.5]);
        mat.rotate_z(slosh * 0.4 * l);
    } else if ctx.item.is_boat && ctx.item.id.contains("chest") {
        let lid_bounce = easings::ease_in_sine(math::abs(walk.sin())) * walk_smoother * 12.0
            + fall.abs() * 15.0;
        state
            .model_animator
            .rotate_x(6, 12, (-lid_bounce.clamp(0.0, 28.0)) as f32, [0.5, 0.6, 0.5]);
    }
}
