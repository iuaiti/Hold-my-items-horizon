//! Player state snapshot ported from `P.java` using Horizon v0.2 `ClientHost`,
//! `EntityRef`, `LivingRef`, and `PlayerRef` APIs.

use mod_api::{ClientHost, Hand, ItemStack};

use crate::item_info::ItemInfo;

/// Frame-local snapshot of the local player's physical and gameplay state.
#[derive(Clone, Debug)]
pub struct PlayerSnapshot {
    pub health: f64,
    pub sneaking: bool,
    pub sprinting: bool,
    pub on_ground: bool,
    pub swimming: bool,
    pub climbing: bool,
    pub crawling: bool,
    pub submerged_in_water: bool,
    pub touching_water: bool,
    pub using_spyglass: bool,
    pub using_riptide: bool,
    pub invisible: bool,
    pub spectator: bool,
    pub has_vehicle: bool,

    pub pos: [f64; 3],
    pub velocity: [f64; 3],
    pub speed: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub age: i32,

    pub using_item: bool,
    pub active_hand: Option<Hand>,
    pub use_ticks: i32,
    pub use_remaining: i32,

    pub main_cooling_down: bool,
    pub off_cooling_down: bool,
    pub swing_count: u64,

    pub main_item: ItemInfo,
    pub off_item: ItemInfo,
}

impl Default for PlayerSnapshot {
    fn default() -> Self {
        Self {
            health: 20.0,
            sneaking: false,
            sprinting: false,
            on_ground: true,
            swimming: false,
            climbing: false,
            crawling: false,
            submerged_in_water: false,
            touching_water: false,
            using_spyglass: false,
            using_riptide: false,
            invisible: false,
            spectator: false,
            has_vehicle: false,
            pos: [0.0; 3],
            velocity: [0.0; 3],
            speed: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            age: 0,
            using_item: false,
            active_hand: None,
            use_ticks: 0,
            use_remaining: 0,
            main_cooling_down: false,
            off_cooling_down: false,
            swing_count: 0,
            main_item: ItemInfo::empty(),
            off_item: ItemInfo::empty(),
        }
    }
}

impl PlayerSnapshot {
    pub fn capture(
        host: &dyn ClientHost,
        player_uuid: u128,
        player_entity_id: i32,
        swing_count: u64,
    ) -> Self {
        let mut snap = Self {
            swing_count,
            ..Self::default()
        };

        let item_use = host.local_item_use();
        if let Some(u) = item_use {
            snap.using_item = true;
            snap.active_hand = Some(u.hand);
            snap.use_ticks = u.ticks_used;
            snap.use_remaining = u.ticks_remaining;
        }

        if let Some(cam) = host.camera() {
            snap.yaw = cam.yaw as f64;
            snap.pitch = cam.pitch as f64;
            snap.pos = cam.position;
        }

        let entity_opt = host
            .entity(player_entity_id as u32)
            .or_else(|| host.camera_entity());

        let mut main_stack: Option<ItemStack> = None;
        let mut off_stack: Option<ItemStack> = None;

        if let Some(entity) = entity_opt {
            if let Some(pos) = entity.position(host) {
                snap.pos = pos;
            }
            if let Some(vel) = entity.velocity(host) {
                snap.velocity = vel;
                snap.speed = (vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]).sqrt();
            }
            if let Some(rot) = entity.rotation(host) {
                snap.yaw = rot[0] as f64;
                snap.pitch = rot[1] as f64;
            }
            snap.on_ground = entity.on_ground(host).unwrap_or(true);
            snap.sneaking = entity.is_crouching(host).unwrap_or(false)
                || entity.is_shift_key_down(host).unwrap_or(false);
            snap.sprinting = entity.is_sprinting(host).unwrap_or(false);
            snap.touching_water = entity.is_in_water(host).unwrap_or(false);
            snap.submerged_in_water = entity
                .eye_fluid(host)
                .flatten()
                .is_some_and(|f| f.contains("water"));
            snap.invisible = entity.is_invisible(host).unwrap_or(false);
            snap.spectator = entity.spectator(host).unwrap_or(false);
            snap.has_vehicle = entity.is_passenger(host).unwrap_or(false);
            snap.age = entity.tick_count(host).unwrap_or(0);

            let pose = entity.pose(host).unwrap_or("standing");
            snap.swimming = pose == "swimming" && snap.touching_water;
            snap.crawling = pose == "swimming" && !snap.touching_water;
            snap.using_riptide = pose == "spin_attack";

            if let Some(living) = host.living(entity) {
                snap.health = living.health(host).unwrap_or(20.0) as f64;
                snap.climbing = living.is_climbing(host).unwrap_or(false);
                main_stack = living.hand_item(host, Hand::Main);
                off_stack = living.hand_item(host, Hand::Off);
            }
        }

        if let Some(player) = host.player(player_uuid) {
            if let Some(ref st) = main_stack {
                let grp = host.item_cooldown_group(st);
                snap.main_cooling_down = player.has_item_cooldown(host, &grp).unwrap_or(false);
            }
            if let Some(ref st) = off_stack {
                let grp = host.item_cooldown_group(st);
                snap.off_cooling_down = player.has_item_cooldown(host, &grp).unwrap_or(false);
            }
        }

        snap.main_item = ItemInfo::from_stack(host, main_stack.as_ref(), snap.use_ticks);
        snap.off_item = ItemInfo::from_stack(host, off_stack.as_ref(), snap.use_ticks);

        if snap.using_item {
            let active_is_spyglass = match snap.active_hand {
                Some(Hand::Main) => snap.main_item.is_of("minecraft:spyglass"),
                Some(Hand::Off) => snap.off_item.is_of("minecraft:spyglass"),
                None => false,
            };
            snap.using_spyglass = active_is_spyglass;
        }

        snap
    }

    pub fn is_item_cooling_down(&self, hand: Hand) -> bool {
        match hand {
            Hand::Main => self.main_cooling_down,
            Hand::Off => self.off_cooling_down,
        }
    }

    pub fn horizontal_speed(&self) -> f64 {
        (self.velocity[0] * self.velocity[0] + self.velocity[2] * self.velocity[2]).sqrt()
    }
}
