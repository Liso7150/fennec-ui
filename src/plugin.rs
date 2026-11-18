//! Everything-is-a-plugin: each optional UI capability is declared as a plugin
//! with a stable id. The shells render composer tools / inspector sections from
//! this registry instead of hard-coding them, so a feature can be disabled (or
//! later supplied by an external plugin) without touching layout code.
//!
//! MVP: the built-in set below is enabled by default; persistence / a toggle UI
//! can layer on top of `Plugin::enabled` later.

/// A declarative UI capability.
#[derive(Clone, Debug)]
pub struct Plugin {
    pub id: &'static str,
    pub name: &'static str,
    pub enabled: bool,
}

/// The built-in feature plugins, all enabled for the MVP.
pub fn default_plugins() -> Vec<Plugin> {
    vec![
        Plugin { id: "attach", name: "File attachments", enabled: true },
        Plugin { id: "vision", name: "Image / vision input", enabled: true },
        Plugin { id: "stt", name: "Speech-to-text (mic)", enabled: true },
        Plugin { id: "tts", name: "Text-to-speech (per message)", enabled: true },
        Plugin { id: "queue", name: "Task queue strip", enabled: true },
        Plugin { id: "inspector", name: "Workspace context inspector", enabled: true },
        Plugin { id: "workspaces", name: "Workspace tree", enabled: true },
    ]
}

/// Whether the plugin with `id` is enabled in `plugins`.
pub fn enabled(plugins: &[Plugin], id: &str) -> bool {
    plugins.iter().any(|p| p.id == id && p.enabled)
}

/// Ids of the enabled plugins (convenience for gating toolbars).
pub fn enabled_ids(plugins: &[Plugin]) -> Vec<&'static str> {
    plugins.iter().filter(|p| p.enabled).map(|p| p.id).collect()
}
