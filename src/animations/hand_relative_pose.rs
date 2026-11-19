//! Relative arm-to-item pose ported from `hand_relative_pose.lua` and
//! `HeldItemRendererMixin.mainHandPose`.

use crate::animations::FrameContext;
use crate::easings;
use crate::item_info::UseAction;
use crate::math::{self, PI, PoseStack};
use crate::state::HmiState;

pub fn apply_hand_relative_pose(
    mat: &mut PoseStack,
    ctx: &FrameContext<'_>,
    state: &mut HmiState,
) {
    let l = if ctx.is_right_arm { 1.0 } else { -1.0 };

    let main_switch = state.registry.get_f64("mainHandSwitch", 1.0);
    let off_switch = state.registry.get_f64("offHandSwitch", 1.0);
    let drink_count = state.registry.get_f64("drinkCount", 0.0);

    // Empty-hand recovery right after drinking
    if ctx.item.empty && drink_count > 0.0 {
        mat.move_x(1.5 * l);
        mat.move_y(-0.3);
        mat.move_z(-0.47);
        mat.rotate_x_pivot(15.0, 0.5 * l, 0.5, 0.5);
        mat.rotate_y_pivot(35.0 * l, 0.5 * l, 0.5, 0.5);
        mat.rotate_z_pivot(-65.0 * l, 0.5 * l, 0.5, 0.5);
        mat.scale(0.9, 0.9, 0.9);
        return;
    }

    let switch_val = if ctx.main_hand {
        main_switch
    } else {
        off_switch
    };
    let switch_anim = easings::ease_in_back(
        (math::clamp(switch_val, 0.09723, 0.60632) * 3.24 * 1.65 - 0.1).sin(),
    );

    let is_tossable_or_small = ctx.item.is_bundle
        || ctx.item.is_of("minecraft:ender_pearl")
        || ctx.item.is_of("minecraft:ender_eye")
        || ctx.item.is_throwable
        || ctx.item.is_music_disc
        || ctx.item.is_nugget
        || ctx.item.is_skull;

    if is_tossable_or_small && ctx.item.use_action != UseAction::Trident {
        mat.rotate_x(10.0 * switch_anim);
        mat.rotate_z(6.0 * switch_anim);
    }

    // Music disc flourish tilt on main-hand switch
    if ctx.item.is_music_disc && ctx.main_hand {
        let disc_tilt = if main_switch < 0.65245 {
            (math::clamp(main_switch, 0.0, 0.16675) * PI * 3.0).sin()
        } else {
            (math::clamp(main_switch, 0.65245, 1.0) * 4.4 - 1.3).sin()
        };
        let disc_jump = (math::clamp(main_switch, 0.52459, 0.85809) * PI * 3.0 - 1.8).sin();
        mat.move_y(0.06 * disc_jump.max(0.0));
        mat.rotate_z(12.0 * disc_tilt * l);
    }
}
