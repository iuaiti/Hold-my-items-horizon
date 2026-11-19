//! Item inspection helpers ported from `I.java`, `JSItems.java`, `JSTags.java`,
//! and `SpearData.java`.

use mod_api::{BlockKey, ClientHost, Identifier, ItemStack};

use crate::state::HmiState;

/// Kinetic spear timing data ported from `com.holdmylua.source.data_structures.SpearData`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpearData {
    pub can_damage: bool,
    pub can_knockback: bool,
    pub can_dismount: bool,
    pub hit_impact: bool,
}

/// Semantic use-action matching vanilla `UseAnim` (`ItemUseAnimation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseAction {
    None,
    Eat,
    Drink,
    Block,
    Bow,
    Trident,
    Crossbow,
    Spyglass,
    TootHorn,
    Brush,
    Spear,
    Bundle,
}

impl UseAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Eat => "eat",
            Self::Drink => "drink",
            Self::Block => "block",
            Self::Bow => "bow",
            Self::Trident => "trident",
            Self::Crossbow => "crossbow",
            Self::Spyglass => "spyglass",
            Self::TootHorn => "toot_horn",
            Self::Brush => "brush",
            Self::Spear => "spear",
            Self::Bundle => "bundle",
        }
    }
}

/// Snapshot of a held item's properties for a single frame.
#[derive(Clone, Debug)]
pub struct ItemInfo {
    pub id: String,
    pub display_name: String,
    pub empty: bool,
    pub block_key: Option<BlockKey>,
    pub use_action: UseAction,
    pub enchanted: bool,
    pub charged_crossbow: bool,
    pub is_sword: bool,
    pub is_axe: bool,
    pub is_pickaxe: bool,
    pub is_shovel: bool,
    pub is_hoe: bool,
    pub is_mace: bool,
    pub is_spear: bool,
    pub is_tool: bool,
    pub is_rod: bool,
    pub is_throwable: bool,
    pub is_lantern: bool,
    pub is_torch: bool,
    pub is_bucket: bool,
    pub is_boat: bool,
    pub is_bundle: bool,
    pub is_map: bool,
    pub is_skull: bool,
    pub is_music_disc: bool,
    pub is_nugget: bool,
    pub is_flower_or_petal: bool,
    pub is_sign: bool,
    pub is_banner: bool,
    pub is_bed: bool,
    pub is_door_or_tall: bool,
    pub is_compass_or_clock: bool,
    pub spear_data: SpearData,
}

impl Default for ItemInfo {
    fn default() -> Self {
        Self::empty()
    }
}

impl ItemInfo {
    pub fn empty() -> Self {
        Self {
            id: "minecraft:air".to_owned(),
            display_name: "Air".to_owned(),
            empty: true,
            block_key: None,
            use_action: UseAction::None,
            enchanted: false,
            charged_crossbow: false,
            is_sword: false,
            is_axe: false,
            is_pickaxe: false,
            is_shovel: false,
            is_hoe: false,
            is_mace: false,
            is_spear: false,
            is_tool: false,
            is_rod: false,
            is_throwable: false,
            is_lantern: false,
            is_torch: false,
            is_bucket: false,
            is_boat: false,
            is_bundle: false,
            is_map: false,
            is_skull: false,
            is_music_disc: false,
            is_nugget: false,
            is_flower_or_petal: false,
            is_sign: false,
            is_banner: false,
            is_bed: false,
            is_door_or_tall: false,
            is_compass_or_clock: false,
            spear_data: SpearData::default(),
        }
    }

    pub fn from_stack(
        host: &dyn ClientHost,
        stack: Option<&ItemStack>,
        ticks_used: i32,
    ) -> Self {
        let Some(stack) = stack else {
            return Self::empty();
        };
        if stack.count() == 0 {
            return Self::empty();
        }
        let item_key = stack.item();
        let id = item_key.id(host);
        if id.is_empty() || id == "minecraft:air" {
            return Self::empty();
        }

        let display_name = host.hover_name(stack);
        let block_key = host.item_block(item_key);
        let nbt_str = stack
            .to_nbt(host)
            .map(|c| format!("{c:?}"))
            .unwrap_or_default();
        let enchanted =
            nbt_str.contains("enchantments") || nbt_str.contains("enchantment_glint_override");

        let path = id.split_once(':').map(|(_, p)| p).unwrap_or(id.as_str());

        let has_tag = |tag_str: &str| -> bool {
            let Ok(ident) = Identifier::parse(tag_str) else {
                return false;
            };
            host.tags()
                .key(&ident)
                .and_then(|tk| host.tags().is_in_tag(item_key, tk))
                .unwrap_or(false)
        };

        let is_sword = has_tag("minecraft:swords") || path.ends_with("_sword");
        let is_axe = has_tag("minecraft:axes") || path.ends_with("_axe");
        let is_pickaxe = has_tag("minecraft:pickaxes") || path.ends_with("_pickaxe");
        let is_shovel = has_tag("minecraft:shovels") || path.ends_with("_shovel");
        let is_hoe = has_tag("minecraft:hoes") || path.ends_with("_hoe");
        let is_mace = path == "mace" || has_tag("c:tools/mace");
        let is_spear = path.ends_with("_spear") || path == "spear";
        let is_tool = is_sword
            || is_axe
            || is_pickaxe
            || is_shovel
            || is_hoe
            || is_mace
            || is_spear
            || has_tag("c:tools")
            || path == "shears"
            || path == "flint_and_steel"
            || path == "brush";
        let is_rod = has_tag("c:rods")
            || path == "stick"
            || path == "blaze_rod"
            || path == "breeze_rod"
            || path == "fishing_rod"
            || path == "carrot_on_a_stick"
            || path == "warped_fungus_on_a_stick"
            || path == "end_rod";
        let is_throwable = matches!(
            path,
            "snowball"
                | "egg"
                | "ender_pearl"
                | "ender_eye"
                | "experience_bottle"
                | "splash_potion"
                | "lingering_potion"
                | "wind_charge"
                | "fire_charge"
        );
        let is_lantern = path.contains("lantern");
        let is_torch = path.contains("torch");
        let is_bucket = path.ends_with("_bucket") || path == "bucket";
        let is_boat = path.ends_with("_boat") || path.ends_with("_raft");
        let is_bundle = has_tag("minecraft:bundles") || path.ends_with("bundle");
        let is_map = path == "filled_map" || path == "map";
        let is_skull = has_tag("minecraft:skulls")
            || path.ends_with("_head")
            || path.ends_with("_skull");
        let is_music_disc = has_tag("c:music_discs") || path.starts_with("music_disc_");
        let is_nugget = has_tag("c:nuggets") || path.ends_with("_nugget");
        let is_flower_or_petal = matches!(path, "pink_petals" | "wildflowers" | "leaf_litter")
            || has_tag("minecraft:flowers");
        let is_sign = path.ends_with("_sign") || has_tag("minecraft:signs");
        let is_banner = path.ends_with("_banner") || has_tag("minecraft:banners");
        let is_bed = path.ends_with("_bed") || has_tag("minecraft:beds");
        let is_door_or_tall = path.ends_with("_door")
            || matches!(
                path,
                "sunflower" | "lilac" | "rose_bush" | "peony" | "tall_grass" | "large_fern" | "pitcher_plant"
            );
        let is_compass_or_clock =
            path == "compass" || path == "recovery_compass" || path == "clock";

        let use_action = infer_use_action(path, is_spear, is_bundle);

        // Check crossbow charged projectiles component via NBT if present
        let charged_crossbow = if path == "crossbow" {
            nbt_str.contains("charged_projectiles") && !nbt_str.contains("charged_projectiles: []")
        } else {
            false
        };

        let spear_data = if is_spear {
            SpearData {
                can_damage: ticks_used >= 4,
                can_knockback: ticks_used >= 8,
                can_dismount: ticks_used >= 14,
                hit_impact: false,
            }
        } else {
            SpearData::default()
        };

        Self {
            id,
            display_name,
            empty: false,
            block_key,
            use_action,
            enchanted,
            charged_crossbow,
            is_sword,
            is_axe,
            is_pickaxe,
            is_shovel,
            is_hoe,
            is_mace,
            is_spear,
            is_tool,
            is_rod,
            is_throwable,
            is_lantern,
            is_torch,
            is_bucket,
            is_boat,
            is_bundle,
            is_map,
            is_skull,
            is_music_disc,
            is_nugget,
            is_flower_or_petal,
            is_sign,
            is_banner,
            is_bed,
            is_door_or_tall,
            is_compass_or_clock,
            spear_data,
        }
    }

    pub fn is_of(&self, target_id: &str) -> bool {
        self.id == target_id
    }

    pub fn is_block(&self) -> bool {
        self.block_key.is_some()
    }

    /// Mirrors `I.shouldTranslateItem(ItemStack)`.
    pub fn should_translate_item(&self) -> bool {
        let base = !self.is_rod
            && !self.is_tool
            && !self.is_mace
            && self.use_action != UseAction::Block;
        base || matches!(
            self.use_action,
            UseAction::Bow | UseAction::Drink | UseAction::Spyglass
        )
    }

    /// Mirrors `HeldItemRendererMixin` block-in-hand eligibility check.
    pub fn should_render_as_block(&self, state: &HmiState) -> bool {
        if !state.settings.render_blocks_3d || self.block_key.is_none() {
            return false;
        }
        if self.is_sign
            || self.is_banner
            || self.is_bed
            || self.is_skull
            || self.use_action == UseAction::Bow
        {
            return false;
        }
        state.render_as_block.get(&self.id).copied().unwrap_or(true)
    }
}

fn infer_use_action(path: &str, is_spear: bool, is_bundle: bool) -> UseAction {
    if path == "bow" {
        UseAction::Bow
    } else if path == "crossbow" {
        UseAction::Crossbow
    } else if path == "trident" {
        UseAction::Trident
    } else if path == "shield" {
        UseAction::Block
    } else if path == "spyglass" {
        UseAction::Spyglass
    } else if path == "goat_horn" {
        UseAction::TootHorn
    } else if path == "brush" {
        UseAction::Brush
    } else if is_spear {
        UseAction::Spear
    } else if is_bundle {
        UseAction::Bundle
    } else if matches!(
        path,
        "potion" | "milk_bucket" | "honey_bottle" | "ominous_bottle"
    ) {
        UseAction::Drink
    } else if is_food_item(path) {
        UseAction::Eat
    } else {
        UseAction::None
    }
}

fn is_food_item(path: &str) -> bool {
    matches!(
        path,
        "apple"
            | "golden_apple"
            | "enchanted_golden_apple"
            | "bread"
            | "porkchop"
            | "cooked_porkchop"
            | "beef"
            | "cooked_beef"
            | "chicken"
            | "cooked_chicken"
            | "mutton"
            | "cooked_mutton"
            | "rabbit"
            | "cooked_rabbit"
            | "cod"
            | "cooked_cod"
            | "salmon"
            | "cooked_salmon"
            | "tropical_fish"
            | "pufferfish"
            | "carrot"
            | "golden_carrot"
            | "potato"
            | "baked_potato"
            | "poisonous_potato"
            | "beetroot"
            | "beetroot_soup"
            | "mushroom_stew"
            | "rabbit_stew"
            | "suspicious_stew"
            | "melon_slice"
            | "sweet_berries"
            | "glow_berries"
            | "chorus_fruit"
            | "dried_kelp"
            | "cookie"
            | "pumpkin_pie"
            | "rotten_flesh"
            | "spider_eye"
    )
}
