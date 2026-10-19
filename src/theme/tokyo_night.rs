//! Tokyo Night, ported from the doom-emacs theme (github.com/doomemacs/themes).

use ratatui::style::Color;

use super::{rgb, Theme};

pub(super) fn build() -> Theme {
    let base0 = rgb(0x414868);
    let base1 = rgb(0x51587a);
    let base8 = rgb(0xc0caf5);
    let fg = rgb(0xa9b1d6);
    let bg = rgb(0x1a1b26);
    let hl_line = rgb(0x292e42);
    let red = rgb(0xf7768e);
    let orange = rgb(0xff9e64);
    let green = rgb(0x73daca);
    let dark_green = rgb(0x9ece6a);
    let yellow = rgb(0xe0af68);
    let blue = rgb(0x7aa2f7);
    let cyan = rgb(0xb4f9f8);
    let bright_cyan = rgb(0xbffaf9);
    let dark_cyan = rgb(0x7dcfff);
    let dark_blue = rgb(0x565f89);
    let magenta = rgb(0xbb9af7);

    Theme {
        // Sections
        section_heading: blue,
        header_bg: dark_blue,
        header_fg: base8,
        change_id: magenta,
        commit_id: blue,
        id_rest: fg,
        bookmark: cyan,
        tag: yellow,
        working_copy: green,
        graph: fg,
        conflict: red,
        empty_marker: green,
        no_description: yellow,
        immutable: bright_cyan,
        root: green,
        file_status: fg,
        log_author: orange,
        log_date: blue,
        op_id: blue,
        // Diffs
        hunk_header: bg,
        hunk_header_bg: rgb(0x404458),
        diff_add: rgb(0x5caea2),
        diff_add_bg: rgb(0x232e36),
        diff_remove: rgb(0xc65e72),
        diff_remove_bg: rgb(0x4a232b),
        // Chrome
        cursor_bg: hl_line,
        search_match: base0,
        bar_bg: rgb(0x161720),
        bar_fg: fg,
        message: dark_cyan,
        warning: yellow,
        error: red,
        success: green,
        // Menus (transient / which-key / help)
        key: red,
        input_prompt: yellow,
        picker_match: cyan,
        picker_marker: yellow,
        picker_selected_bg: base0,
        picker_selected_fg: Color::Reset,
        picker_count: base1,
        menu_title: magenta,
        command: blue,
        transient_argument: dark_green,
        help_border: dark_cyan,
    }
}
