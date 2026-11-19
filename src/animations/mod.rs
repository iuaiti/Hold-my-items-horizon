//! Animation pipeline coordinator ported from `HeldItemRendererMixin.java`
//! and Hold My Items 5.1.1's `pack_test` scripts (`hand_pose`, `hand_relative_pose`,
//! `item_pose`, `item_model`).

pub mod hand_pose;
pub mod hand_relative_pose;
pub mod item_model;
pub mod item_pose;

use mod_api::Hand;

use crate::item_info::ItemInfo;
use crate::math::PoseStack;
use crate::player_info::PlayerSnapshot;
use crate::state::HmiState;

/// Per-frame context passed through all four HMI animation stages (mirrors `LuaContext.java`).
pub struct FrameContext<'a> {
    pub is_right_arm: bool,
    pub main_hand: bool,
    pub hand: Hand,
    pub item: &'a ItemInfo,
    pub player: &'a PlayerSnapshot,
    pub delta_time: f64,
    pub swing_progress: f64,
    pub equip_progress: f64,
    pub main_hand_swing_progress: f64,
    pub off_hand_swing_progress: f64,
    pub main_hand_switch_event: bool,
    pub off_hand_switch_event: bool,
    pub swing_main_toggle: bool,
    pub swing_off_toggle: bool,
    pub interact: bool,
    pub block_breaking: bool,
}

/// Output of evaluating the full Hold My Items animation pipeline for one hand.
pub struct EvaluatedHandPose {
    /// Combined scene + hand/item matrix stack.
    pub pose: PoseStack,
    /// Relative hand-only matrix stack (for empty/arm transforms).
    pub hand_pose: PoseStack,
    /// Whether this hand should be hidden (e.g. offhand while drawing a bow/crossbow).
    pub hide_hand: bool,
}

/// Runs `scenePoseMain`, `mainHandPose`, `itemPose`, and `itemModel` in order,
/// matching `HeldItemRendererMixin.renderOverhaul`.
pub fn evaluate_hand_animation(
    ctx: &FrameContext<'_>,
    state: &mut HmiState,
) -> EvaluatedHandPose {
    // Hide offhand while two-handed bow/crossbow is active in main hand (and vice versa)
    let other_item = if ctx.main_hand {
        &ctx.player.off_item
    } else {
        &ctx.player.main_item
    };
    let other_using_two_handed = ctx.player.using_item
        && ctx.player.active_hand != Some(ctx.hand)
        && (other_item.use_action == crate::item_info::UseAction::Bow
            || (other_item.use_action == crate::item_info::UseAction::Crossbow
                && !other_item.charged_crossbow));

    if other_using_two_handed {
        return EvaluatedHandPose {
            pose: PoseStack::new(),
            hand_pose: PoseStack::new(),
            hide_hand: true,
        };
    }

    let mut scene = PoseStack::new();
    hand_pose::apply_scene_pose(&mut scene, ctx, state);

    let mut hand_rel = scene.clone();
    hand_relative_pose::apply_hand_relative_pose(&mut hand_rel, ctx, state);

    let mut item_stack = scene.clone();
    if !ctx.item.empty {
        item_pose::apply_item_pose(&mut item_stack, ctx, state);
        state.model_animator.clear();
        item_model::apply_item_model_pose(&mut item_stack, ctx, state);
    }

    // Store matrices for particle/reference consumers (LuaTestHMI.matricesMain / matricesOff)
    if ctx.is_right_arm {
        state.matrices_main = item_stack.current();
    } else {
        state.matrices_off = item_stack.current();
    }

    EvaluatedHandPose {
        pose: if ctx.item.empty { hand_rel.clone() } else { item_stack },
        hand_pose: hand_rel,
        hide_hand: false,
    }
}
