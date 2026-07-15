//! Data-driven transient menus (Magit's popup system). A transient is pure
//! data: groups of switches and actions. While one is open it captures all
//! keys; actions collect the enabled switches into CLI flags.

use std::collections::{BTreeMap, BTreeSet};

use ratatui::crossterm::event::KeyCode;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};

use crate::command::Menu;
use crate::keymap::KeyPress;
use crate::theme::Theme;

/// What an action ultimately runs. The app maps these to jj invocations,
/// usually against the revision at point (falling back to `@`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransientAction {
    /// `jj describe REV` with $EDITOR.
    Describe,
    /// `jj commit` with $EDITOR (describe @ + new child).
    Commit,
    /// `jj new REV` — new child of the revision at point.
    NewChild,
    /// `jj new --insert-after REV`.
    NewAfter,
    /// `jj new --insert-before REV`.
    NewBefore,
    /// `jj squash -r REV` — squash into the parent.
    Squash,
    /// `jj squash --into REV` — squash the working copy into REV.
    SquashInto,
    /// `jj rebase -r REV -d <picked>` — rebase the single revision.
    RebaseRev,
    /// `jj rebase -s REV -d <picked>` — rebase REV and descendants.
    RebaseSource,
    /// `jj rebase -b REV -d <picked>` — rebase the whole branch.
    RebaseBranch,
    /// `jj bookmark create <name> -r REV`.
    BookmarkCreate,
    /// `jj bookmark set <picked> -r REV`.
    BookmarkSet,
    /// `jj bookmark delete <picked>`.
    BookmarkDelete,
    /// `jj bookmark rename <picked> <name>`.
    BookmarkRename,
    /// `jj bookmark track <name@remote>`.
    BookmarkTrack,
    /// `jj bookmark untrack <name@remote>`.
    BookmarkUntrack,
    /// `jj bookmark forget <picked>`.
    BookmarkForget,
    /// `jj git push` (all tracked bookmarks).
    Push,
    /// `jj git push --bookmark <picked>`.
    PushBookmark,
    /// `jj git push --change REV` (auto-creates a bookmark).
    PushChange,
    /// `jj git fetch`.
    Fetch,
    /// `jj git fetch --all-remotes`.
    FetchAllRemotes,
    /// Log the default revset in a new buffer.
    LogDefault,
    /// Log all revisions (`-r ::`).
    LogAll,
    /// Prompt for a revset, then log it.
    LogRevset,
}

#[derive(Debug, Clone, Copy)]
pub enum Item {
    /// A boolean flag, toggled on/off.
    Switch {
        key: &'static str,
        flag: &'static str,
        desc: &'static str,
    },
    /// A flag that takes a value (e.g. `-n`). Selecting it prompts for
    /// the value; the flag and value are passed as two separate arguments.
    Arg {
        key: &'static str,
        flag: &'static str,
        desc: &'static str,
    },
    Action {
        key: &'static str,
        desc: &'static str,
        action: TransientAction,
    },
}

impl Item {
    fn key(&self) -> &'static str {
        match self {
            Item::Switch { key, .. } | Item::Arg { key, .. } | Item::Action { key, .. } => key,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GroupDef {
    pub title: &'static str,
    pub items: &'static [Item],
}

#[derive(Debug, Clone, Copy)]
pub struct TransientDef {
    pub title: &'static str,
    pub groups: &'static [GroupDef],
    /// Switch flags enabled when the menu opens (Magit's `:value`).
    pub defaults: &'static [&'static str],
    /// Sets of mutually exclusive switches (Magit's `:incompatible`):
    /// enabling one clears the others in its set.
    pub incompatible: &'static [&'static [&'static str]],
}

pub static DESCRIBE: TransientDef = TransientDef {
    title: "Describe",
    defaults: &[],
    incompatible: &[],
    groups: &[
        GroupDef {
            title: "Arguments",
            items: &[Item::Switch {
                key: "-a",
                flag: "--reset-author",
                desc: "Reset the author to yourself",
            }],
        },
        GroupDef {
            title: "Actions",
            items: &[Item::Action {
                key: "c",
                desc: "Describe the revision at point",
                action: TransientAction::Describe,
            }],
        },
    ],
};

pub static COMMIT: TransientDef = TransientDef {
    title: "Commit",
    defaults: &[],
    incompatible: &[],
    groups: &[
        GroupDef {
            title: "Arguments",
            items: &[Item::Switch {
                key: "-a",
                flag: "--reset-author",
                desc: "Reset the author to yourself",
            }],
        },
        GroupDef {
            title: "Actions",
            items: &[Item::Action {
                key: "c",
                desc: "Commit the working copy (describe + new)",
                action: TransientAction::Commit,
            }],
        },
    ],
};

pub static NEW: TransientDef = TransientDef {
    title: "New change",
    defaults: &[],
    incompatible: &[],
    groups: &[
        GroupDef {
            title: "Arguments",
            items: &[Item::Switch {
                key: "-E",
                flag: "--no-edit",
                desc: "Do not switch to the new change",
            }],
        },
        GroupDef {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "o",
                    desc: "New child of the revision at point",
                    action: TransientAction::NewChild,
                },
                Item::Action {
                    key: "a",
                    desc: "Insert after the revision at point",
                    action: TransientAction::NewAfter,
                },
                Item::Action {
                    key: "b",
                    desc: "Insert before the revision at point",
                    action: TransientAction::NewBefore,
                },
            ],
        },
    ],
};

pub static SQUASH: TransientDef = TransientDef {
    title: "Squash",
    defaults: &[],
    incompatible: &[],
    groups: &[
        GroupDef {
            title: "Arguments",
            items: &[
                Item::Switch {
                    key: "-k",
                    flag: "--keep-emptied",
                    desc: "Keep the emptied source revision",
                },
                Item::Switch {
                    key: "-u",
                    flag: "--use-destination-message",
                    desc: "Keep the destination's description",
                },
            ],
        },
        GroupDef {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "s",
                    desc: "Squash the revision at point into its parent",
                    action: TransientAction::Squash,
                },
                Item::Action {
                    key: "t",
                    desc: "Squash the working copy into the revision at point",
                    action: TransientAction::SquashInto,
                },
            ],
        },
    ],
};

pub static REBASE: TransientDef = TransientDef {
    title: "Rebase",
    defaults: &[],
    incompatible: &[],
    groups: &[
        GroupDef {
            title: "Arguments",
            items: &[Item::Switch {
                key: "-e",
                flag: "--skip-emptied",
                desc: "Abandon revisions that become empty",
            }],
        },
        GroupDef {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "r",
                    desc: "Rebase the revision at point",
                    action: TransientAction::RebaseRev,
                },
                Item::Action {
                    key: "s",
                    desc: "Rebase the revision and its descendants",
                    action: TransientAction::RebaseSource,
                },
                Item::Action {
                    key: "b",
                    desc: "Rebase the whole branch",
                    action: TransientAction::RebaseBranch,
                },
            ],
        },
    ],
};

pub static BOOKMARK: TransientDef = TransientDef {
    title: "Bookmark",
    defaults: &[],
    incompatible: &[],
    groups: &[GroupDef {
        title: "Actions",
        items: &[
            Item::Action {
                key: "c",
                desc: "Create at the revision at point",
                action: TransientAction::BookmarkCreate,
            },
            Item::Action {
                key: "s",
                desc: "Set (move) to the revision at point",
                action: TransientAction::BookmarkSet,
            },
            Item::Action {
                key: "d",
                desc: "Delete",
                action: TransientAction::BookmarkDelete,
            },
            Item::Action {
                key: "r",
                desc: "Rename",
                action: TransientAction::BookmarkRename,
            },
            Item::Action {
                key: "t",
                desc: "Track a remote bookmark",
                action: TransientAction::BookmarkTrack,
            },
            Item::Action {
                key: "u",
                desc: "Untrack a remote bookmark",
                action: TransientAction::BookmarkUntrack,
            },
            Item::Action {
                key: "f",
                desc: "Forget (local and remote refs)",
                action: TransientAction::BookmarkForget,
            },
        ],
    }],
};

pub static PUSH: TransientDef = TransientDef {
    title: "Push",
    defaults: &[],
    incompatible: &[],
    groups: &[
        GroupDef {
            title: "Arguments",
            items: &[
                Item::Switch {
                    key: "-N",
                    flag: "--allow-new",
                    desc: "Allow pushing new bookmarks",
                },
                Item::Switch {
                    key: "-d",
                    flag: "--deleted",
                    desc: "Push deleted bookmarks",
                },
                Item::Switch {
                    key: "-n",
                    flag: "--dry-run",
                    desc: "Only show what would change",
                },
            ],
        },
        GroupDef {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "p",
                    desc: "Push all tracked bookmarks",
                    action: TransientAction::Push,
                },
                Item::Action {
                    key: "b",
                    desc: "Push a bookmark",
                    action: TransientAction::PushBookmark,
                },
                Item::Action {
                    key: "c",
                    desc: "Push the change at point (creates a bookmark)",
                    action: TransientAction::PushChange,
                },
            ],
        },
    ],
};

pub static FETCH: TransientDef = TransientDef {
    title: "Fetch",
    defaults: &[],
    incompatible: &[],
    groups: &[GroupDef {
        title: "Actions",
        items: &[
            Item::Action {
                key: "f",
                desc: "Fetch from the default remote",
                action: TransientAction::Fetch,
            },
            Item::Action {
                key: "a",
                desc: "Fetch from all remotes",
                action: TransientAction::FetchAllRemotes,
            },
        ],
    }],
};

pub static LOG: TransientDef = TransientDef {
    title: "Log",
    defaults: &[],
    incompatible: &[],
    groups: &[
        GroupDef {
            title: "Arguments",
            items: &[Item::Arg {
                key: "-n",
                flag: "-n",
                desc: "Limit number of revisions",
            }],
        },
        GroupDef {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "l",
                    desc: "Log the default revset",
                    action: TransientAction::LogDefault,
                },
                Item::Action {
                    key: "a",
                    desc: "Log all revisions",
                    action: TransientAction::LogAll,
                },
                Item::Action {
                    key: "o",
                    desc: "Log another revset",
                    action: TransientAction::LogRevset,
                },
            ],
        },
    ],
};

/// Resolve a `Command::Transient(menu)` to its definition. A new menu adds
/// one arm here and nothing in `App::dispatch`.
pub fn menu_def(menu: Menu) -> &'static TransientDef {
    match menu {
        Menu::Describe => &DESCRIBE,
        Menu::Commit => &COMMIT,
        Menu::New => &NEW,
        Menu::Squash => &SQUASH,
        Menu::Rebase => &REBASE,
        Menu::Bookmark => &BOOKMARK,
        Menu::Push => &PUSH,
        Menu::Fetch => &FETCH,
        Menu::Log => &LOG,
    }
}

/// A currently-open transient: the definition plus toggled switches and the
/// multi-char key input buffer (switch keys like "-a" are two keystrokes).
#[derive(Debug, Clone)]
pub struct TransientState {
    pub def: &'static TransientDef,
    pub enabled: BTreeSet<&'static str>,
    /// Value arguments (`-n` → "50"), set via a value prompt.
    pub values: BTreeMap<&'static str, String>,
    pub pending: String,
}

/// Outcome of feeding one key to an open transient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransientResult {
    /// Key consumed (switch toggled or prefix pending); keep the menu open.
    Consumed,
    /// Prompt for this value argument's value; the menu stays open.
    Prompt {
        flag: &'static str,
        desc: &'static str,
    },
    /// Run this action with these collected CLI flags; menu closes.
    Invoke(TransientAction, Vec<String>),
    /// Close without running anything.
    Cancel,
    /// Key didn't match any item.
    Unbound,
}

impl TransientState {
    pub fn new(def: &'static TransientDef) -> Self {
        Self {
            def,
            enabled: def.defaults.iter().copied().collect(),
            values: BTreeMap::new(),
            pending: String::new(),
        }
    }

    /// Set (or clear, when empty) a value argument's value.
    pub fn set_value(&mut self, flag: &'static str, value: String) {
        if value.is_empty() {
            self.values.remove(flag);
        } else {
            self.values.insert(flag, value);
        }
    }

    pub fn args(&self) -> Vec<String> {
        let mut out: Vec<String> = self.enabled.iter().map(|f| f.to_string()).collect();
        for (flag, val) in &self.values {
            out.push(flag.to_string());
            out.push(val.clone());
        }
        out
    }

    pub fn on_key(&mut self, kp: &KeyPress) -> TransientResult {
        if kp.code == KeyCode::Esc
            || (kp.code == KeyCode::Char('g')
                && kp
                    .mods
                    .contains(ratatui::crossterm::event::KeyModifiers::CONTROL))
        {
            if self.pending.is_empty() {
                return TransientResult::Cancel;
            }
            self.pending.clear();
            return TransientResult::Consumed;
        }
        let KeyCode::Char(c) = kp.code else {
            return TransientResult::Unbound;
        };
        self.pending.push(c);

        let items = || self.def.groups.iter().flat_map(|g| g.items.iter());
        if let Some(item) = items().find(|i| i.key() == self.pending) {
            self.pending.clear();
            match *item {
                Item::Switch { flag, .. } => {
                    if !self.enabled.remove(flag) {
                        self.enabled.insert(flag);
                        // Enabling a switch clears the rest of its
                        // incompatible set.
                        for set in self.def.incompatible {
                            if set.contains(&flag) {
                                for other in set.iter().filter(|o| **o != flag) {
                                    self.enabled.remove(other);
                                }
                            }
                        }
                    }
                    TransientResult::Consumed
                }
                Item::Arg { flag, desc, .. } => TransientResult::Prompt { flag, desc },
                Item::Action { action, .. } => TransientResult::Invoke(action, self.args()),
            }
        } else if items().any(|i| i.key().starts_with(&self.pending)) {
            TransientResult::Consumed
        } else {
            self.pending.clear();
            TransientResult::Unbound
        }
    }

    /// Lines for the bottom panel.
    pub fn render_lines(&self, t: &Theme) -> Vec<Line<'static>> {
        let mut out = Vec::new();
        for group in self.def.groups {
            out.push(Line::from(Span::styled(
                group.title.to_string(),
                Style::new().fg(t.menu_title).add_modifier(Modifier::BOLD),
            )));
            for item in group.items {
                match *item {
                    Item::Switch { key, flag, desc } => {
                        let on = self.enabled.contains(flag);
                        let flag_style = if on {
                            Style::new().fg(t.command).bold()
                        } else {
                            Style::new().dim()
                        };
                        out.push(Line::from(vec![
                            Span::raw(" "),
                            Span::styled(format!("{key:<4}"), Style::new().fg(t.key)),
                            Span::styled(format!("{flag:<26}"), flag_style),
                            Span::raw(desc.to_string()),
                        ]));
                    }
                    Item::Arg { key, flag, desc } => {
                        let (shown, style) = match self.values.get(flag) {
                            Some(v) => (format!("{flag} {v}"), Style::new().fg(t.command).bold()),
                            None => (flag.to_string(), Style::new().dim()),
                        };
                        out.push(Line::from(vec![
                            Span::raw(" "),
                            Span::styled(format!("{key:<4}"), Style::new().fg(t.key)),
                            Span::styled(format!("{shown:<26}"), style),
                            Span::raw(desc.to_string()),
                        ]));
                    }
                    Item::Action { key, desc, .. } => {
                        out.push(Line::from(vec![
                            Span::raw(" "),
                            Span::styled(format!("{key:<4}"), Style::new().fg(t.key)),
                            Span::raw(desc.to_string()),
                        ]));
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn key(c: char) -> KeyPress {
        KeyPress::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn switch_key_is_a_two_key_sequence() {
        let mut st = TransientState::new(&SQUASH);
        assert_eq!(st.on_key(&key('-')), TransientResult::Consumed);
        assert_eq!(st.on_key(&key('k')), TransientResult::Consumed);
        assert!(st.enabled.contains("--keep-emptied"));
        // Toggling off again.
        st.on_key(&key('-'));
        st.on_key(&key('k'));
        assert!(st.enabled.is_empty());
    }

    #[test]
    fn action_collects_enabled_flags() {
        let mut st = TransientState::new(&SQUASH);
        st.on_key(&key('-'));
        st.on_key(&key('u'));
        match st.on_key(&key('s')) {
            TransientResult::Invoke(TransientAction::Squash, args) => {
                assert_eq!(args, vec!["--use-destination-message"]);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn value_arg_prompts_then_collects_flag_and_value() {
        let mut st = TransientState::new(&LOG);
        assert_eq!(st.on_key(&key('-')), TransientResult::Consumed);
        assert_eq!(
            st.on_key(&key('n')),
            TransientResult::Prompt {
                flag: "-n",
                desc: "Limit number of revisions",
            }
        );
        st.set_value("-n", "50".into());
        match st.on_key(&key('l')) {
            TransientResult::Invoke(TransientAction::LogDefault, args) => {
                assert_eq!(args, vec!["-n", "50"]);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn empty_value_clears_the_arg() {
        let mut st = TransientState::new(&LOG);
        st.set_value("-n", "50".into());
        assert_eq!(st.args(), vec!["-n", "50"]);
        st.set_value("-n", String::new());
        assert!(st.args().is_empty());
    }

    #[test]
    fn switch_and_action_sharing_a_letter_stay_distinct() {
        // NEW has both the "-E" switch prefix "-" and actions; the pending
        // prefix must keep two-key sequences separate from actions.
        let mut st = TransientState::new(&NEW);
        st.on_key(&key('-'));
        st.on_key(&key('E'));
        assert!(st.enabled.contains("--no-edit"));
        match st.on_key(&key('b')) {
            TransientResult::Invoke(TransientAction::NewBefore, args) => {
                assert_eq!(args, vec!["--no-edit"]);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn esc_cancels() {
        let mut st = TransientState::new(&PUSH);
        let esc = KeyPress::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(st.on_key(&esc), TransientResult::Cancel);
    }

    #[test]
    fn unknown_key_is_unbound() {
        let mut st = TransientState::new(&PUSH);
        assert_eq!(st.on_key(&key('z')), TransientResult::Unbound);
        assert!(st.pending.is_empty());
    }
}
