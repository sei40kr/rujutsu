//! Color scheme. Every color the UI uses is a named role on `Theme`, so the
//! whole look is overridable from `[colors]` in config.toml. Values accept
//! ratatui's color syntax: names ("red", "lightblue"), hex ("#3a3a3a"), and
//! 256-color indexes ("42").

use std::str::FromStr;

use ratatui::style::Color;

// One theme per file: each `<name>.rs` here exposes `pub(super) fn build() ->
// Theme` and gets one row in `PRESETS` below.
mod tokyo_night;

macro_rules! theme {
    ($($field:ident: $default:expr => $key:literal),+ $(,)?) => {
        #[derive(Debug, Clone)]
        pub struct Theme {
            $(pub $field: Color),+
        }

        impl Default for Theme {
            fn default() -> Self {
                Self { $($field: $default),+ }
            }
        }

        impl Theme {
            /// Override one role by its config key. `Err` for an unknown key
            /// or an unparsable color, with a message for the warning list.
            pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
                let color = Color::from_str(value)
                    .map_err(|_| format!("colors: invalid color {value:?} for {key:?}"))?;
                match key {
                    $($key => self.$field = color,)+
                    _ => return Err(format!("colors: unknown key {key:?}")),
                }
                Ok(())
            }
        }
    };
}

theme! {
    // Sections
    section_heading: Color::Cyan     => "section-heading",   // group titles
    header_bg:       Color::Blue     => "header-bg",         // log header bar background
    header_fg:       Color::White    => "header-fg",         // log header bar text
    change_id:       Color::Magenta  => "change-id",         // change id prefix
    commit_id:       Color::Blue     => "commit-id",
    bookmark:        Color::Magenta  => "bookmark",          // bookmark names in log
    tag:             Color::Yellow   => "tag",
    working_copy:    Color::Green    => "working-copy",      // the "@" marker / row
    graph:           Color::DarkGray => "graph",             // log graph edges
    conflict:        Color::Red      => "conflict",
    empty_marker:    Color::Green    => "empty-marker",      // "(empty)" tag
    no_description:  Color::Yellow   => "no-description",    // "(no description set)"
    immutable:       Color::DarkGray => "immutable",         // ◆ nodes / immutable info
    file_status:     Color::Blue     => "file-status",       // "modified" prefix
    log_author:      Color::Blue     => "log-author",        // log margin author
    log_date:        Color::Gray     => "log-date",          // log margin date
    op_id:           Color::Yellow   => "op-id",             // operation ids
    // Diffs
    hunk_header:     Color::Cyan     => "hunk-header",
    diff_add:        Color::Green    => "diff-add",
    diff_remove:     Color::Red      => "diff-remove",
    // Chrome
    cursor_bg:       Color::DarkGray => "cursor-bg",
    search_match:    Color::Yellow   => "search-match",   // bg of matched text
    bar_bg:          Color::Black    => "bar-bg",
    bar_fg:          Color::Gray     => "bar-fg",
    message:         Color::Cyan     => "message",
    warning:         Color::Yellow   => "warning",           // busy indicator, confirm
    error:           Color::Red      => "error",
    success:         Color::Green    => "success",
    // Menus (transient / which-key / help)
    key:             Color::Yellow   => "key",
    input_prompt:    Color::Yellow   => "input-prompt",       // minibuffer "> "
    picker_match:    Color::Cyan     => "picker-match",       // fuzzy-matched chars
    picker_marker:   Color::Yellow   => "picker-marker",      // "▸" on the selection
    picker_selected_bg: Color::DarkGray => "picker-selected-bg",
    picker_selected_fg: Color::Reset    => "picker-selected-fg",
    picker_count:    Color::DarkGray => "picker-count",       // "12/45" hint
    menu_title:      Color::Magenta  => "menu-title",
    command:         Color::Cyan     => "command",           // command names in help
    help_border:     Color::Cyan     => "help-border",
}

// ---------------------------------------------------------------------------
// Built-in presets
//
// A preset is a named `Theme` constructor living in its own file. `theme =
// "<name>"` in config.toml picks the base theme; `[colors]` overrides still
// layer on top. Adding one is two edits: a new `src/theme/<name>.rs` exposing
// `pub(super) fn build() -> Theme`, and a row in `PRESETS`.
// ---------------------------------------------------------------------------

impl Theme {
    /// The built-in preset for `name`, or `None` if there is no such preset.
    pub fn preset(name: &str) -> Option<Theme> {
        PRESETS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, build)| build())
    }
}

/// A built-in preset: its config name and a constructor for the full theme.
pub type Preset = (&'static str, fn() -> Theme);

/// Every built-in preset. The single source of truth for `Theme::preset` and
/// for enumerating names in warnings/help.
pub const PRESETS: &[Preset] = &[("tokyo-night", tokyo_night::build)];

/// Comma-separated preset names, for warning messages.
pub fn preset_names() -> String {
    PRESETS
        .iter()
        .map(|(n, _)| *n)
        .collect::<Vec<_>>()
        .join(", ")
}

/// `0xRRGGBB` literal to a truecolor `Color`. Keeps the per-theme palette
/// tables readable; shared by every file in this module.
pub(super) const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_accepts_names_hex_and_indexed() {
        let mut t = Theme::default();
        t.set("diff-add", "blue").unwrap();
        assert_eq!(t.diff_add, Color::Blue);
        t.set("cursor-bg", "#3a3a3a").unwrap();
        assert_eq!(t.cursor_bg, Color::Rgb(0x3a, 0x3a, 0x3a));
        t.set("key", "42").unwrap();
        assert_eq!(t.key, Color::Indexed(42));
    }

    #[test]
    fn set_rejects_unknown_key_and_bad_color() {
        let mut t = Theme::default();
        assert!(t.set("no-such-role", "red").is_err());
        assert!(t.set("diff-add", "not-a-color").is_err());
    }

    #[test]
    fn preset_resolves_known_and_rejects_unknown() {
        let t = Theme::preset("tokyo-night").expect("tokyo-night preset exists");
        assert_eq!(t.diff_add, Color::Rgb(0x73, 0xda, 0xca)); // vc-added green
        assert!(Theme::preset("no-such-theme").is_none());
        assert!(preset_names().contains("tokyo-night"));
    }

    #[test]
    fn colors_override_layers_on_top_of_a_preset() {
        // A `[colors]` entry must still win over the chosen preset.
        let mut t = Theme::preset("tokyo-night").unwrap();
        t.set("diff-add", "#010203").unwrap();
        assert_eq!(t.diff_add, Color::Rgb(1, 2, 3));
    }
}
