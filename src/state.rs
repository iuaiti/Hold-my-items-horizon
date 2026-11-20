//! Persistent mod state ported from `GlobalsStorage`, `LivingEntityMixin`,
//! `CameraMixin`, `LuaTestHMI`, and `KeyBindManager`.

use std::collections::{HashMap, HashSet};
use glam::Mat4;
use mod_api::{ClientHost, Hand};

use crate::debug_overlay::DebugTextRenderer;
use crate::lua_loader::LoadedLuaScripts;
use crate::model_animator::ModelPartAnimator;
use crate::particles::Particle;

pub const MOD_ID: &str = "holdmyitems";

/// Mod settings registered on Horizon's Options > Mods screen.
#[derive(Clone, Debug)]
pub struct HmiSettings {
    pub enabled: bool,
    pub x_offset: f64,
    pub y_offset: f64,
    pub z_offset: f64,
    pub camera_shake: bool,
    pub particles: bool,
    pub render_blocks_3d: bool,
    pub sound_effects: bool,
    pub debug_overlay: bool,
}

impl Default for HmiSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
            camera_shake: true,
            particles: true,
            render_blocks_3d: true,
            sound_effects: true,
            debug_overlay: false,
        }
    }
}

impl HmiSettings {
    /// Pulls the latest saved values from Horizon's `ConfigClientHost`.
    pub fn refresh(&mut self, host: &dyn ClientHost) {
        if let Some(v) = host.setting_bool("enabled") {
            self.enabled = v;
        }
        if let Some(v) = host.setting_f64("x_offset") {
            self.x_offset = v.clamp(-1.0, 1.0);
        }
        if let Some(v) = host.setting_f64("y_offset") {
            self.y_offset = v.clamp(-1.0, 1.0);
        }
        if let Some(v) = host.setting_f64("z_offset") {
            self.z_offset = v.clamp(-1.0, 1.0);
        }
        if let Some(v) = host.setting_bool("camera_shake") {
            self.camera_shake = v;
        }
        if let Some(v) = host.setting_bool("particles") {
            self.particles = v;
        }
        if let Some(v) = host.setting_bool("render_blocks_3d") {
            self.render_blocks_3d = v;
        }
        if let Some(v) = host.setting_bool("sound_effects") {
            self.sound_effects = v;
        }
        if let Some(v) = host.setting_bool("debug_overlay") {
            self.debug_overlay = v;
        }
    }
}

/// Camera rotation and translation offset requested by `C.setCamRot` / `C.setCamPos`.
#[derive(Clone, Copy, Debug, Default)]
pub struct CameraOffset {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub pos: [f32; 3],
}

/// Per-hand swing, interact, and block-breaking state ported from `LivingEntityMixin`
/// and `MinecraftClientMixin`.
#[derive(Clone, Debug, Default)]
pub struct LivingSwingTracker {
    pub interact_main: bool,
    pub interact_off: bool,
    pub block_breaking: bool,

    pub main_swing_ticks: i32,
    pub main_swinging: bool,
    pub main_swing_progress: f32,
    pub last_main_swing_progress: f32,

    pub off_swing_ticks: i32,
    pub off_swinging: bool,
    pub off_swing_progress: f32,
    pub last_off_swing_progress: f32,

    pub main_swing_count: u64,
    pub swing_main_toggle: bool,
    pub swing_off_toggle: bool,

    pub prev_main_item: String,
    pub prev_off_item: String,
    pub main_hand_switch_event: bool,
    pub off_hand_switch_event: bool,
}

impl LivingSwingTracker {
    pub fn reset_main_hand_swing(&mut self, interact: bool, is_breaking_block: bool) {
        self.main_swing_ticks = 0;
        self.main_swinging = true;
        self.swing_main_toggle = !self.swing_main_toggle;
        self.main_swing_count = self.main_swing_count.wrapping_add(1);
        self.interact_main = interact;
        if is_breaking_block {
            self.block_breaking = true;
        }
    }

    pub fn reset_off_hand_swing(&mut self, interact: bool) {
        self.off_swing_ticks = 0;
        self.off_swinging = true;
        self.swing_off_toggle = !self.swing_off_toggle;
        self.interact_off = interact;
    }

    pub fn on_swing_hand(&mut self, hand: Hand, duration: i32, is_breaking_block: bool) {
        let dur = duration.max(1);
        match hand {
            Hand::Off => {
                if !self.off_swinging || self.off_swing_ticks >= dur / 2 || self.off_swing_ticks < 0
                {
                    self.off_swing_ticks = -1;
                    self.off_swinging = true;
                }
            }
            Hand::Main => {
                if !self.main_swinging
                    || self.main_swing_ticks >= dur / 2
                    || self.main_swing_ticks < 0
                {
                    self.main_swing_ticks = -1;
                    self.main_swinging = true;
                    if is_breaking_block {
                        self.block_breaking = true;
                    }
                }
            }
        }
    }

    /// Advances swing timers once per client tick (`LivingEntityMixin::tick`).
    pub fn tick(&mut self, main_duration: i32, off_duration: i32) {
        self.last_off_swing_progress = self.off_swing_progress;
        self.last_main_swing_progress = self.main_swing_progress;

        let main_dur = main_duration.max(1);
        if self.main_swinging {
            self.main_swing_ticks += 1;
            if self.main_swing_ticks >= main_dur {
                self.main_swing_ticks = 0;
                self.main_swinging = false;
            }
        } else {
            self.interact_main = false;
            self.block_breaking = false;
            self.main_swing_ticks = 0;
        }
        self.main_swing_progress = self.main_swing_ticks as f32 / main_dur as f32;

        let off_dur = off_duration.max(1);
        if self.off_swinging {
            self.off_swing_ticks += 1;
            if self.off_swing_ticks >= off_dur {
                self.off_swing_ticks = 0;
                self.off_swinging = false;
            }
        } else {
            self.interact_off = false;
            self.off_swing_ticks = 0;
        }
        self.off_swing_progress = self.off_swing_ticks as f32 / off_dur as f32;
    }

    pub fn main_progress(&self, partial: f32) -> f32 {
        let mut delta = self.main_swing_progress - self.last_main_swing_progress;
        if delta < 0.0 {
            delta += 1.0;
        }
        self.last_main_swing_progress + delta * partial
    }

    pub fn off_progress(&self, partial: f32) -> f32 {
        let mut delta = self.off_swing_progress - self.last_off_swing_progress;
        if delta < 0.0 {
            delta += 1.0;
        }
        self.last_off_swing_progress + delta * partial
    }
}

/// Persistent numeric/boolean animation variables (`GlobalsStorage.registry`).
#[derive(Clone, Debug, Default)]
pub struct AnimRegistry {
    pub floats: HashMap<String, f64>,
    pub bools: HashMap<String, bool>,
}

impl AnimRegistry {
    #[inline]
    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        self.floats.get(key).copied().unwrap_or(default)
    }

    #[inline]
    pub fn set_f64(&mut self, key: &str, value: f64) {
        self.floats.insert(key.to_owned(), value);
    }

    #[inline]
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.bools.get(key).copied().unwrap_or(default)
    }

    #[inline]
    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.bools.insert(key.to_owned(), value);
    }

    pub fn clear(&mut self) {
        self.floats.clear();
        self.bools.clear();
    }
}

/// Complete runtime state of Hold My Items in Horizon v0.2.
#[derive(Debug)]
pub struct HmiState {
    pub settings: HmiSettings,
    pub settings_dirty: bool,
    pub delta_time: f32,
    pub tick_progress: f32,

    pub camera: CameraOffset,
    pub swing: LivingSwingTracker,
    pub registry: AnimRegistry,

    /// Ported `GlobalsStorage` lookup tables.
    pub render_as_block: HashMap<String, bool>,
    pub translate_item: HashMap<String, bool>,
    pub apply_block_rotation: HashMap<String, bool>,
    pub item_swing_speed: HashMap<String, i32>,
    pub use_duration: HashMap<String, i32>,
    pub using_item: HashMap<String, bool>,

    /// Last main/off hand matrices (for screen-space and item-space particles).
    pub matrices_main: Mat4,
    pub matrices_off: Mat4,

    /// Active first-person particles.
    pub particles: Vec<Particle>,
    /// Per-element model part animator.
    pub model_animator: ModelPartAnimator,
    /// HUD debug text queue.
    pub debug_text: DebugTextRenderer,

    /// Currently held raw GLFW / key codes for `KeyBindManager::is_key_pressed`.
    pub pressed_keys: HashSet<u32>,
    pub inspect_held: bool,

    /// Loaded resource-pack Lua scripts (and addons).
    pub lua_scripts: LoadedLuaScripts,

    /// Pending local sounds queued during `&dyn ClientHost` hooks.
    pub pending_sounds: Vec<(String, f32, f32)>,
}

impl Default for HmiState {
    fn default() -> Self {
        let mut state = Self {
            settings: HmiSettings::default(),
            settings_dirty: true,
            delta_time: 1.0 / 60.0,
            tick_progress: 0.0,
            camera: CameraOffset::default(),
            swing: LivingSwingTracker::default(),
            registry: AnimRegistry::default(),
            render_as_block: HashMap::new(),
            translate_item: HashMap::new(),
            apply_block_rotation: HashMap::new(),
            item_swing_speed: HashMap::new(),
            use_duration: HashMap::new(),
            using_item: HashMap::new(),
            matrices_main: Mat4::IDENTITY,
            matrices_off: Mat4::IDENTITY,
            particles: Vec::new(),
            model_animator: ModelPartAnimator::default(),
            debug_text: DebugTextRenderer::default(),
            pressed_keys: HashSet::new(),
            inspect_held: false,
            lua_scripts: LoadedLuaScripts::default(),
            pending_sounds: Vec::new(),
        };
        state.init_default_item_tables();
        state
    }
}

impl HmiState {
    /// Seeds default per-item swing speeds and render overrides from `item_pose.lua`.
    pub fn init_default_item_tables(&mut self) {
        self.render_as_block
            .insert("minecraft:torch".to_owned(), true);
        self.render_as_block
            .insert("minecraft:soul_torch".to_owned(), true);
        self.render_as_block
            .insert("minecraft:redstone_torch".to_owned(), true);
        self.render_as_block
            .insert("minecraft:lantern".to_owned(), true);
        self.render_as_block
            .insert("minecraft:soul_lantern".to_owned(), true);
        self.render_as_block
            .insert("minecraft:bell".to_owned(), true);

        // Default custom swing speeds matching HMI 5.1.1 pack_test
        for sword in [
            "minecraft:wooden_sword",
            "minecraft:stone_sword",
            "minecraft:iron_sword",
            "minecraft:golden_sword",
            "minecraft:diamond_sword",
            "minecraft:netherite_sword",
        ] {
            self.item_swing_speed.insert(sword.to_owned(), 9);
        }
        for axe in [
            "minecraft:wooden_axe",
            "minecraft:stone_axe",
            "minecraft:iron_axe",
            "minecraft:golden_axe",
            "minecraft:diamond_axe",
            "minecraft:netherite_axe",
        ] {
            self.item_swing_speed.insert(axe.to_owned(), 11);
        }
        self.item_swing_speed
            .insert("minecraft:mace".to_owned(), 12);
        self.item_swing_speed
            .insert("minecraft:trident".to_owned(), 10);
    }

    pub fn clear_on_reload(&mut self) {
        self.render_as_block.clear();
        self.translate_item.clear();
        self.apply_block_rotation.clear();
        self.registry.clear();
        self.use_duration.clear();
        self.using_item.clear();
        self.init_default_item_tables();
    }

    pub fn swing_speed_for(&self, item_id: &str) -> i32 {
        self.item_swing_speed.get(item_id).copied().unwrap_or(10)
    }

    pub fn queue_sound(&mut self, id: &str, volume: f32, pitch: f32) {
        if self.settings.sound_effects {
            self.pending_sounds
                .push((id.to_owned(), volume.clamp(0.0, 2.0), pitch.clamp(0.5, 2.0)));
        }
    }
}
