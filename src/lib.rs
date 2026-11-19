//! Hold My Items 5.1.1 — Horizon v0.2 SDK Port
//!
//! Target SDK: `sdk-f00125f61f794aaa3d4e950d301f353b35b6ee3d` (`content_api = "0.2"`,
//! `package_format = 6`, Minecraft `26.3`, Rust `1.99.1`).
//!
//! Ported from Fabric/Java (`com.holdmylua.source.*`) to native/wasm Horizon `mod_api`.

pub mod animations;
pub mod debug_overlay;
pub mod easings;
pub mod item_info;
pub mod lua_loader;
pub mod math;
pub mod model_animator;
pub mod particles;
pub mod player_info;
pub mod state;

use std::sync::Mutex;

use mod_api::{
    self, CameraShakeEvent, ClientHost, ClientResourcesReloadedEvent, ClientSetupEvent,
    ClientTickPostEvent, Flow, Hand, Hooks, InputKeyEvent, KeyAction, KeyConflictContext,
    KeyRegistration, MouseButtonPreEvent, Registrar, RegistrationContext, RenderFramePreEvent,
    RenderGuiPostEvent, RenderHandEvent, SettingChangedEvent, SettingSpec,
    SoundPlay,
};

use crate::animations::{evaluate_hand_animation, FrameContext};
use crate::item_info::ItemInfo;
use crate::particles::{draw_particles, RenderSpace};
use crate::player_info::PlayerSnapshot;
use crate::state::{HmiState, MOD_ID};

/// Root mod struct registered via `#[mod_api::horizon_mod]`.
#[derive(Default)]
pub struct HoldMyItems {
    state: Mutex<HmiState>,
}

#[mod_api::horizon_mod]
impl Hooks for HoldMyItems {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let specs = [
            SettingSpec::toggle("enabled", "setting.holdmyitems.enabled", true)
                .tooltip("setting.holdmyitems.enabled.tooltip"),
            SettingSpec::range(
                "x_offset",
                "setting.holdmyitems.x_offset",
                -1.0,
                1.0,
                0.05,
                0.0,
            )
            .tooltip("setting.holdmyitems.x_offset.tooltip"),
            SettingSpec::range(
                "y_offset",
                "setting.holdmyitems.y_offset",
                -1.0,
                1.0,
                0.05,
                0.0,
            )
            .tooltip("setting.holdmyitems.y_offset.tooltip"),
            SettingSpec::range(
                "z_offset",
                "setting.holdmyitems.z_offset",
                -1.0,
                1.0,
                0.05,
                0.0,
            )
            .tooltip("setting.holdmyitems.z_offset.tooltip"),
            SettingSpec::toggle("camera_shake", "setting.holdmyitems.camera_shake", true)
                .tooltip("setting.holdmyitems.camera_shake.tooltip"),
            SettingSpec::toggle("particles", "setting.holdmyitems.particles", true)
                .tooltip("setting.holdmyitems.particles.tooltip"),
            SettingSpec::toggle(
                "render_blocks_3d",
                "setting.holdmyitems.render_blocks_3d",
                true,
            )
            .tooltip("setting.holdmyitems.render_blocks_3d.tooltip"),
            SettingSpec::toggle("sound_effects", "setting.holdmyitems.sound_effects", true)
                .tooltip("setting.holdmyitems.sound_effects.tooltip"),
            SettingSpec::toggle("debug_overlay", "setting.holdmyitems.debug_overlay", false)
                .tooltip("setting.holdmyitems.debug_overlay.tooltip"),
        ];

        for spec in specs {
            if let Err(err) = r.register_setting(spec) {
                log::warn!("[HoldMyItems] Failed to register setting: {err}");
            }
        }

        // Default inspect keybind: J (GLFW 74 in original respackopts.json5)
        r.register_key_mapping(
            KeyRegistration::new("inspect", "key.keyboard.j")
                .label("key.holdmyitems.inspect")
                .category("key.categories.holdmyitems")
                .context(KeyConflictContext::InGame),
        );
    }

    fn client_setup(&self, host: &mut dyn ClientHost, _event: &mut ClientSetupEvent) {
        if let Ok(mut state) = self.state.lock() {
            state.settings.refresh(host);
            lua_loader::reload_lua_resources(host, &mut state);
        }
        log::info!("[HoldMyItems] Initialized for Horizon v0.2 (SDK f00125f6).");
    }

    fn client_resources_reloaded(
        &self,
        host: &mut dyn ClientHost,
        _event: &mut ClientResourcesReloadedEvent,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state.settings.refresh(host);
            lua_loader::reload_lua_resources(host, &mut state);
        }
    }

    fn setting_changed(&self, host: &mut dyn ClientHost, _event: &mut SettingChangedEvent<'_>) {
        if let Ok(mut state) = self.state.lock() {
            state.settings.refresh(host);
        }
    }

    fn render_frame_pre(&self, host: &mut dyn ClientHost, event: &mut RenderFramePreEvent) {
        if !host.on(MOD_ID) {
            return;
        }
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        // Mirrors GameRendererMixin.deltaTime
        if host.is_paused() {
            state.delta_time = 0.0;
        } else {
            state.delta_time = event.elapsed.as_secs_f32().min(0.05);
        }
        state.tick_progress = event.world_partial_tick;

        // Flush any queued local sounds (e.g. sword slashy.ogg)
        let sounds: Vec<(String, f32, f32)> = state.pending_sounds.drain(..).collect();
        drop(state);

        for (id, volume, pitch) in sounds {
            let mut play = SoundPlay::ui(&id);
            play.volume = volume;
            play.pitch = pitch;
            let _ = host.play_local_sound(&play, 0);
        }
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPostEvent) {
        if !host.on(MOD_ID) {
            return;
        }
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        state.inspect_held = host.is_down(MOD_ID, "inspect");

        // Advance custom swing timers (LivingEntityMixin.tick)
        let main_dur = state.swing_speed_for(&state.swing.prev_main_item);
        let off_dur = state.swing_speed_for(&state.swing.prev_off_item);
        state.swing.tick(main_dur, off_dur);
    }

    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        if !host.on(MOD_ID) {
            return;
        }
        if let Ok(mut state) = self.state.lock() {
            match event.action {
                KeyAction::Press | KeyAction::Repeat => {
                    state.pressed_keys.insert(event.raw_code);
                }
                KeyAction::Release => {
                    state.pressed_keys.remove(&event.raw_code);
                }
            }
        }
    }

    fn mouse_button_pre(&self, host: &mut dyn ClientHost, event: &mut MouseButtonPreEvent) -> Flow {
        if !host.on(MOD_ID) || !event.pressed || !host.playing() {
            return Flow::Continue;
        }
        if let Ok(mut state) = self.state.lock() {
            if event.button == 0 {
                // Left click: attack or block break (MinecraftClientMixin.doAttackMix)
                let breaking = host.crosshair_target().is_some();
                state.swing.reset_main_hand_swing(false, breaking);
            } else if event.button == 1 {
                // Right click: item/block use (MinecraftClientMixin.doItemUse)
                state.swing.reset_main_hand_swing(true, false);
            }
        }
        Flow::Continue
    }


    fn render_hand(&self, host: &dyn ClientHost, event: &mut RenderHandEvent<'_, '_>) -> Flow {
        if !host.on(MOD_ID) {
            return Flow::Continue;
        }
        let Ok(mut state) = self.state.lock() else {
            return Flow::Continue;
        };
        if !state.settings.enabled {
            return Flow::Continue;
        }

        let player = PlayerSnapshot::capture(
            host,
            event.render.player,
            event.render.player_entity_id,
            state.swing.main_swing_count,
        );

        // Skip overhaul while looking through spyglass or in spectator mode
        if player.using_spyglass || player.spectator {
            return Flow::Continue;
        }

        let is_main_hand = event.render.hand == Hand::Main;
        let is_right_arm = is_main_hand;
        let item = ItemInfo::from_stack(host, event.render.stack, player.use_ticks);

        // Track item switch events (HeldItemRendererMixin.prevMainHand / prevOffHand)
        if is_main_hand {
            state.swing.main_hand_switch_event = item.id != state.swing.prev_main_item;
        } else {
            state.swing.off_hand_switch_event = item.id != state.swing.prev_off_item;
        }

        let partial = event.render.partial;
        let main_swing = state.swing.main_progress(partial) as f64;
        let off_swing = state.swing.off_progress(partial) as f64;
        let swing_progress = if is_main_hand {
            main_swing.max(event.render.swing as f64)
        } else {
            off_swing.max(event.render.swing as f64)
        };
        let interact = if is_main_hand {
            state.swing.interact_main
        } else {
            state.swing.interact_off
        };

        let ctx = FrameContext {
            is_right_arm,
            main_hand: is_main_hand,
            hand: event.render.hand,
            item: &item,
            player: &player,
            delta_time: state.delta_time as f64,
            swing_progress,
            equip_progress: event.render.equip as f64,
            main_hand_swing_progress: main_swing,
            off_hand_swing_progress: off_swing,
            main_hand_switch_event: state.swing.main_hand_switch_event,
            off_hand_switch_event: state.swing.off_hand_switch_event,
            swing_main_toggle: state.swing.swing_main_toggle,
            swing_off_toggle: state.swing.swing_off_toggle,
            interact,
            block_breaking: state.swing.block_breaking,
        };

        let evaluated = evaluate_hand_animation(&ctx, &mut state);

        // Update previous item IDs for next frame switch detection
        if is_main_hand {
            state.swing.prev_main_item = item.id.clone();
        } else {
            state.swing.prev_off_item = item.id.clone();
        }

        if evaluated.hide_hand {
            return Flow::Cancel;
        }

        // Apply pose transform to Horizon's HandRender
        let (trans, rot, scale) = evaluated.pose.decompose_for_hand_render();
        event.render.translation[0] += trans[0];
        event.render.translation[1] += trans[1];
        event.render.translation[2] += trans[2];
        event.render.rotation[0] += rot[0];
        event.render.rotation[1] += rot[1];
        event.render.rotation[2] += rot[2];
        event.render.scale *= scale;

        // Draw first-person particles in ITEM space and SCREEN space
        if state.settings.particles && !state.particles.is_empty() {
            let dt = state.delta_time;
            let age = player.age;

            event.draw.push();
            evaluated.pose.apply_to_draw(event.draw);
            draw_particles(
                &mut state.particles,
                event.draw,
                RenderSpace::Item,
                event.render.hand,
                dt,
                age,
            );
            event.draw.pop();

            event.draw.push();
            draw_particles(
                &mut state.particles,
                event.draw,
                RenderSpace::Screen,
                event.render.hand,
                dt,
                age,
            );
            event.draw.pop();
        }

        if state.settings.debug_overlay && is_main_hand {
            let line1 = format!(
                "[HMI v0.2] item={} action={}",
                item.id,
                item.use_action.as_str()
            );
            let line2 = format!(
                "swing={:.2} equip={:.2} particles={} scripts={}",
                swing_progress,
                event.render.equip,
                state.particles.len(),
                state.lua_scripts.loaded_count
            );
            state.debug_text.out(line1);
            state.debug_text.out(line2);
        }

        Flow::Continue
    }

    fn camera_shake(&self, host: &dyn ClientHost, event: &mut CameraShakeEvent<'_>) {
        if !host.on(MOD_ID) {
            return;
        }
        let Ok(state) = self.state.lock() else {
            return;
        };
        if !state.settings.enabled || !state.settings.camera_shake {
            return;
        }
        event.shake.pitch += state.camera.pitch;
        event.shake.yaw += state.camera.yaw;
        event.shake.roll += state.camera.roll;
        event.shake.offset[0] += state.camera.pos[0];
        event.shake.offset[1] += state.camera.pos[1];
        event.shake.offset[2] += state.camera.pos[2];
    }

    fn render_gui_post(&self, host: &dyn ClientHost, event: &mut RenderGuiPostEvent<'_>) {
        if !host.on(MOD_ID) {
            return;
        }
        if let Ok(mut state) = self.state.lock() {
            if !state.debug_text.lines().is_empty() {
                state.debug_text.render_and_clear(event.draw);
            }
        }
    }
}
