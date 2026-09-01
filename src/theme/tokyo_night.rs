//! Tokyo Night, ported from the doom-emacs theme (github.com/doomemacs/themes).
//!
//! Roles are mapped to the colors doom-themes gives the corresponding Magit
//! faces (`doom-themes-base.el` resolved against this theme's palette), with
//! jj-specific roles (change ids, bookmarks, operations) slotted in by their
//! default ANSI hue — so a rujutsu buffer reads like a Magit buffer under
//! doom-tokyo-night.

use ratatui::style::Color;

use super::{rgb, Theme};

pub(super) fn build() -> Theme {
    // Palette (doom variable names in comments).
    let bg_alt = rgb(0x13141c);
    let base0 = rgb(0x414868); // region / selection
    let base1 = rgb(0x51587a); // comments
    let base8 = rgb(0xc0caf5);
    let fg = rgb(0xa9b1d6);
    let hl_line = rgb(0x292e42); // current-line background
    let red = rgb(0xf7768e);
    let orange = rgb(0xff9e64);
    let green = rgb(0x73daca); // vc-added / success
    let yellow = rgb(0xe0af68);
    let blue = rgb(0x7aa2f7); // magit-section-heading / -log-date
    let cyan = rgb(0xb4f9f8); // highlight / magit-branch-local
    let dark_cyan = rgb(0x7dcfff);
    let dark_blue = rgb(0x565f89); // magit-header-line background
    let magenta = rgb(0xbb9af7);
    let violet = rgb(0x9aa5ce); // magit-diff-hunk-heading / -filename

    Theme {
        // Sections
        section_heading: blue, // magit-section-heading
        header_bg: dark_blue,  // magit-header-line
        header_fg: base8,      //  "
        change_id: magenta,    // jj's magenta change-id prefix
        commit_id: blue,       // jj's blue commit ids
        bookmark: cyan,        // magit-branch-local
        tag: yellow,           // magit-tag
        working_copy: green,   // the "@" marker / row
        graph: base1,          // log graph edges (comments)
        conflict: red,
        empty_marker: green,    // "(empty)" tag
        no_description: yellow, // "(no description set)"
        immutable: base1,       // ◆ nodes (comments)
        file_status: blue,      // "modified" prefix
        log_author: orange,     // magit-log-author
        log_date: blue,         // magit-log-date
        op_id: yellow,          // operation ids
        // Diffs
        hunk_header: violet, // magit-diff-hunk-heading
        diff_add: green,     // vc-added / magit-diffstat-added
        diff_remove: red,    // vc-deleted / magit-diffstat-removed
        // Chrome
        cursor_bg: hl_line,
        search_match: base0, // region / selection
        bar_bg: bg_alt,
        bar_fg: fg,
        message: dark_cyan,
        warning: yellow,
        error: red,     // magit-process-ng inherits error
        success: green, // magit-process-ok inherits success
        // Menus (transient / which-key / help)
        key: yellow,
        input_prompt: yellow,
        picker_match: cyan,
        picker_marker: yellow,
        picker_selected_bg: base0, // region / selection
        picker_selected_fg: Color::Reset,
        picker_count: base1, // comments
        menu_title: magenta,
        command: dark_cyan,
        help_border: dark_cyan,
    }
}
