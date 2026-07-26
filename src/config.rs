//! User configuration: `$XDG_CONFIG_HOME/rujutsu/config.toml`.
//!
//! ```toml
//! scrolloff = 3
//! theme = "tokyo-night"  # a built-in preset (see theme::PRESETS); top-level,
//!                        # so it must precede any [table] header
//!
//! [keys.global]
//! "g"   = "refresh"
//! "P p" = "push"      # space-separated key sequences are supported
//!
//! [keys.status]
//! "s" = "squash"
//!
//! [keys.op-log]
//! "R" = "op-restore"
//!
//! [colors]            # role names: see src/theme/mod.rs
//! diff-add  = "green"  # layered on top of the chosen preset
//! cursor-bg = "#3a3a3a"
//! key       = "42"    # 256-color index
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

use crate::command;
use crate::keymap::{parse_keys, Keymaps, PaneKind};

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub scrolloff: Option<usize>,
    /// Revset for the log section of the status buffer and the default log
    /// buffer. `None` uses jj's configured default (`revsets.log`).
    pub log_revset: Option<String>,
    #[serde(default)]
    pub keys: KeysConfig,
    /// Name of a built-in theme preset (see `theme::PRESETS`). `[colors]`
    /// entries layer on top of it.
    pub theme: Option<String>,
    #[serde(default)]
    pub colors: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct KeysConfig {
    #[serde(default)]
    pub global: HashMap<String, String>,
    #[serde(default)]
    pub status: HashMap<String, String>,
    #[serde(default)]
    pub log: HashMap<String, String>,
    #[serde(default)]
    pub revision: HashMap<String, String>,
    #[serde(default, rename = "op-log")]
    pub op_log: HashMap<String, String>,
}

pub fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("rujutsu").join("config.toml"))
}

/// Load the config file if present. Parse failures are reported as warnings
/// rather than aborting startup.
pub fn load() -> (Config, Vec<String>) {
    let mut warnings = Vec::new();
    let Some(path) = config_path() else {
        return (Config::default(), warnings);
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return (Config::default(), warnings);
    };
    match toml::from_str(&text) {
        Ok(cfg) => (cfg, warnings),
        Err(e) => {
            warnings.push(format!("config error in {}: {e}", path.display()));
            (Config::default(), warnings)
        }
    }
}

/// Merge user bindings over the defaults. Unknown commands or bad key specs
/// become warnings.
pub fn apply_keys(cfg: &Config, keymaps: &mut Keymaps, warnings: &mut Vec<String>) {
    let mut apply = |bindings: &HashMap<String, String>, map: &mut crate::keymap::Keymap| {
        for (spec, cmd_name) in bindings {
            let Some(cmd) = command::by_name(cmd_name) else {
                warnings.push(format!("config: unknown command {cmd_name:?}"));
                continue;
            };
            match parse_keys(spec) {
                Ok(seq) => map.insert(&seq, cmd),
                Err(e) => warnings.push(format!("config: {e}")),
            }
        }
    };
    apply(&cfg.keys.global, &mut keymaps.global);
    for (bindings, kind) in [
        (&cfg.keys.status, PaneKind::Status),
        (&cfg.keys.log, PaneKind::Log),
        (&cfg.keys.revision, PaneKind::Revision),
        (&cfg.keys.op_log, PaneKind::OpLog),
    ] {
        apply(bindings, keymaps.local.entry(kind).or_default());
    }
}

/// The base theme: a built-in preset if `theme = "<name>"` names one, else the
/// default. An unknown preset name becomes a warning and falls back to default.
/// `[colors]` overrides are layered on top afterwards via [`apply_colors`].
pub fn base_theme(cfg: &Config, warnings: &mut Vec<String>) -> crate::theme::Theme {
    use crate::theme::Theme;
    let Some(name) = cfg.theme.as_deref() else {
        return Theme::default();
    };
    Theme::preset(name).unwrap_or_else(|| {
        warnings.push(format!(
            "theme: unknown preset {name:?} (available: {})",
            crate::theme::preset_names()
        ));
        Theme::default()
    })
}

/// Override theme roles from `[colors]`. Bad keys/values become warnings.
pub fn apply_colors(cfg: &Config, theme: &mut crate::theme::Theme, warnings: &mut Vec<String>) {
    for (key, value) in &cfg.colors {
        if let Err(e) = theme.set(key, value) {
            warnings.push(e);
        }
    }
}
