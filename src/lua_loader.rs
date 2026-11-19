//! Resource-pack Lua script loader and preprocessor ported from
//! `LuaAnimationResourceLoader.java`, `LuaScriptCache.java`, and `ScriptHolder.java`.
//!
//! Uses Horizon v0.2's `host.resource(PackType::ClientResources, &id)` to read
//! `assets/minecraft/holdmyitems/*.lua` from active resource packs, preprocesses
//! `global.<name> = <val>;` and `${placeholder}` directives, and extracts
//! declarative table overrides (`renderAsBlock`, `itemSwingSpeed`, `translateItem`,
//! `applyBlockRotation`) into `HmiState`.

use std::collections::HashSet;
use mod_api::{ClientHost, Identifier, PackType};

use crate::state::{HmiSettings, HmiState};

#[derive(Clone, Debug, Default)]
pub struct LoadedLuaScripts {
    pub hand_pose: String,
    pub item_pose: String,
    pub hand_relative_pose: String,
    pub item_model: String,
    pub hand_addon: String,
    pub hand_relative_addon: String,
    pub item_addon: String,
    pub item_model_addon: String,
    pub loaded_count: usize,
}

/// Reloads all Hold My Items Lua scripts from the active client resource pack stack.
pub fn reload_lua_resources(host: &dyn ClientHost, state: &mut HmiState) {
    state.clear_on_reload();

    let mut scripts = LoadedLuaScripts::default();
    let settings = state.settings.clone();

    let mut load = |path: &str| -> String {
        let Ok(id) = Identifier::from_parts("minecraft", path) else {
            return String::new();
        };
        match host.resource(PackType::ClientResources, &id) {
            Some(bytes) => match String::from_utf8(bytes) {
                Ok(raw) => {
                    scripts.loaded_count += 1;
                    preprocess_script(&raw, &settings)
                }
                Err(_) => String::new(),
            },
            None => String::new(),
        }
    };

    scripts.hand_pose = load("holdmyitems/hand_pose.lua");
    scripts.item_pose = load("holdmyitems/item_pose.lua");
    scripts.hand_relative_pose = load("holdmyitems/hand_relative_pose.lua");
    scripts.item_model = load("holdmyitems/item_model.lua");
    scripts.hand_addon = load("holdmyitems/hand_addon.lua");
    scripts.hand_relative_addon = load("holdmyitems/hand_relative_addon.lua");
    scripts.item_addon = load("holdmyitems/item_addon.lua");
    scripts.item_model_addon = load("holdmyitems/item_model_addon.lua");

    // Apply declarative overrides from loaded scripts (e.g. itemSwingSpeed:put, renderAsBlock:put)
    for src in [
        &scripts.item_pose,
        &scripts.item_addon,
        &scripts.hand_pose,
        &scripts.hand_addon,
    ] {
        extract_table_overrides(src, state);
    }

    log::info!(
        "[HoldMyItems] Reloaded {} Lua animation script(s) from resource packs.",
        scripts.loaded_count
    );
    state.lua_scripts = scripts;
}

/// Exact port of `LuaAnimationResourceLoader.preprocessScript(String)`.
///
/// 1. Substitutes `${xOffset}`, `${yOffset}`, `${zOffset}`, `${inspectKeybind}` (or `0` for unknown).
/// 2. Rewrites `global.<var> = <expr>;` into `local <var> = registry:getOrDefault('<var>', <expr>)`
///    and appends persistence statements `registry:put('<var>', <var>)`.
pub fn preprocess_script(script: &str, settings: &HmiSettings) -> String {
    if script.is_empty() {
        return String::new();
    }

    let mut persist_vars: Vec<String> = Vec::new();
    let mut persist_set: HashSet<String> = HashSet::new();
    let mut out = String::with_capacity(script.len() + 256);

    for raw_line in script.lines() {
        let line = substitute_placeholders(raw_line, settings);
        if let Some((rewritten, var_name)) = rewrite_global_decl(&line) {
            if persist_set.insert(var_name.clone()) {
                persist_vars.push(var_name);
            }
            out.push_str(&rewritten);
        } else {
            out.push_str(&line);
        }
        out.push('\n');
    }

    if !persist_vars.is_empty() {
        out.push_str("\n-- PERSIST VARIABLES\n");
        for var in persist_vars {
            out.push_str(&format!("registry:put('{var}', {var})\n"));
        }
    }

    out
}

fn substitute_placeholders(line: &str, settings: &HmiSettings) -> String {
    if !line.contains("${") {
        return line.to_owned();
    }
    let mut result = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find('}') {
            let key = &after[..end];
            match key {
                "xOffset" => result.push_str(&format!("{:.4}", settings.x_offset)),
                "yOffset" => result.push_str(&format!("{:.4}", settings.y_offset)),
                "zOffset" => result.push_str(&format!("{:.4}", settings.z_offset)),
                "inspectKeybind" => result.push_str("74"),
                _ => result.push('0'),
            }
            rest = &after[end + 1..];
        } else {
            result.push_str("${");
            rest = after;
        }
    }
    result.push_str(rest);
    result
}

fn rewrite_global_decl(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("--") {
        return None;
    }
    let idx = line.find("global.")?;
    let after_prefix = &line[idx + 7..];
    let eq_idx = after_prefix.find('=')?;
    let var_name = after_prefix[..eq_idx].trim();
    if var_name.is_empty() || !var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let after_eq = &after_prefix[eq_idx + 1..];
    let semi_idx = after_eq.find(';')?;
    let value = after_eq[..semi_idx].trim();
    let indent = &line[..idx];
    let trailing = &after_eq[semi_idx + 1..];
    let rewritten = format!(
        "{indent}local {var_name} = registry:getOrDefault('{var_name}', {value}){trailing}"
    );
    Some((rewritten, var_name.to_owned()))
}

/// Parses `renderAsBlock:put("id", bool)`, `itemSwingSpeed:put("id", int)`,
/// `translateItem:put("id", bool)`, and `applyBlockRotation:put("id", bool)` calls
/// from active resource-pack Lua scripts.
fn extract_table_overrides(script: &str, state: &mut HmiState) {
    for raw_line in script.lines() {
        let line = raw_line.trim();
        if line.starts_with("--") {
            continue;
        }
        if let Some((k, v)) = parse_put_bool(line, "renderAsBlock:put(") {
            state.render_as_block.insert(k, v);
        } else if let Some((k, v)) = parse_put_bool(line, "translateItem:put(") {
            state.translate_item.insert(k, v);
        } else if let Some((k, v)) = parse_put_bool(line, "applyBlockRotation:put(") {
            state.apply_block_rotation.insert(k, v);
        } else if let Some((k, v)) = parse_put_int(line, "itemSwingSpeed:put(") {
            state.item_swing_speed.insert(k, v);
        }
    }
}

fn parse_put_args<'a>(line: &'a str, prefix: &str) -> Option<(&'a str, &'a str)> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest.find(')')?;
    let inner = &rest[..end];
    let (k, v) = inner.split_once(',')?;
    let key = k.trim().trim_matches('"').trim_matches('\'');
    if key.is_empty() {
        return None;
    }
    Some((key, v.trim()))
}

fn parse_put_bool(line: &str, prefix: &str) -> Option<(String, bool)> {
    let (k, v) = parse_put_args(line, prefix)?;
    let b = match v {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    Some((k.to_owned(), b))
}

fn parse_put_int(line: &str, prefix: &str) -> Option<(String, i32)> {
    let (k, v) = parse_put_args(line, prefix)?;
    let n: i32 = v.parse().ok()?;
    Some((k.to_owned(), n))
}
