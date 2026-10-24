//! Data-driven transient menus (Magit's popup system). A menu is pure data:
//! groups of switches and actions over a typed set of choices (`S`), which
//! is the jj-vocabulary options type the actions need. Switches flip fields
//! of `S`; an action turns the current `S` into a `TransientAction`. No CLI
//! flag survives past this module — the labels that look like flags are
//! display text, as in Magit.

use std::num::NonZeroUsize;

use ratatui::crossterm::event::KeyCode;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};

use crate::command::Menu;
use crate::jj::op::{
    Author, Checkout, Deletions, DiffTool, EmptiedCommits, EmptiedSource, Fetch, Fileset, Invalid,
    Placement, PushOptions, RebaseKind, RestoreDescendants, Revset, RunMode, SquashDescription,
    SquashOptions,
};
use crate::keymap::KeyPress;
use crate::theme::Theme;

/// What an action asks for, minus whatever depends on the cursor (the
/// revision at point, a picked bookmark), which the receiver supplies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransientAction {
    Describe(Author),
    Commit(Author),
    New(Placement, Checkout),
    /// Squash the revision at point into its parent.
    Squash(SquashOptions),
    /// Squash the working copy into the revision at point.
    SquashInto(SquashOptions),
    Rebase(RebaseKind, EmptiedCommits),
    BookmarkCreate,
    BookmarkSet,
    BookmarkDelete,
    BookmarkRename,
    BookmarkTrack,
    BookmarkUntrack,
    BookmarkForget,
    /// Push every tracked bookmark.
    Push(PushOptions),
    /// Push a picked bookmark.
    PushBookmark(PushOptions),
    /// Push the change at point under an auto-created bookmark.
    PushChange(PushOptions),
    Fetch(Fetch),
    LogDefault(Option<NonZeroUsize>),
    LogAll(Option<NonZeroUsize>),
    LogRevset(Option<NonZeroUsize>),
    Restore(RestoreChoices),
    /// Undo the changes in the revision at point.
    RestoreChangesIn(RestoreChoices),
}

pub enum Item<S: 'static> {
    Switch {
        key: &'static str,
        label: &'static str,
        desc: &'static str,
        toggle: fn(&mut S),
        is_on: fn(&S) -> bool,
    },
    /// A choice that takes a typed value. Selecting it prompts; the text is
    /// parsed by `set` (empty clears) and `shown` renders the current value.
    Arg {
        key: &'static str,
        label: &'static str,
        desc: &'static str,
        set: fn(&mut S, &str) -> Result<(), String>,
        shown: fn(&S) -> Option<String>,
    },
    Action {
        key: &'static str,
        desc: &'static str,
        build: fn(&S) -> TransientAction,
    },
}

impl<S> Item<S> {
    fn key(&self) -> &'static str {
        match self {
            Item::Switch { key, .. } | Item::Arg { key, .. } | Item::Action { key, .. } => key,
        }
    }
}

pub struct Group<S: 'static> {
    pub title: &'static str,
    pub items: &'static [Item<S>],
}

pub struct MenuDef<S: 'static> {
    pub title: &'static str,
    pub groups: &'static [Group<S>],
}

fn flip_author(a: &mut Author) {
    *a = match a {
        Author::Keep => Author::ResetToCurrentUser,
        Author::ResetToCurrentUser => Author::Keep,
    };
}

const RESET_AUTHOR: Item<Author> = Item::Switch {
    key: "-a",
    label: "--reset-author",
    desc: "Reset the author to yourself",
    toggle: flip_author,
    is_on: |a| *a == Author::ResetToCurrentUser,
};

pub static DESCRIBE: MenuDef<Author> = MenuDef {
    title: "Describe",
    groups: &[
        Group {
            title: "Arguments",
            items: &[RESET_AUTHOR],
        },
        Group {
            title: "Actions",
            items: &[Item::Action {
                key: "c",
                desc: "Describe the revision at point",
                build: |a| TransientAction::Describe(*a),
            }],
        },
    ],
};

pub static COMMIT: MenuDef<Author> = MenuDef {
    title: "Commit",
    groups: &[
        Group {
            title: "Arguments",
            items: &[RESET_AUTHOR],
        },
        Group {
            title: "Actions",
            items: &[Item::Action {
                key: "c",
                desc: "Commit the working copy (describe + new)",
                build: |a| TransientAction::Commit(*a),
            }],
        },
    ],
};

pub static NEW: MenuDef<Checkout> = MenuDef {
    title: "New change",
    groups: &[
        Group {
            title: "Arguments",
            items: &[Item::Switch {
                key: "-E",
                label: "--no-edit",
                desc: "Do not switch to the new change",
                toggle: |c| {
                    *c = match c {
                        Checkout::MoveWorkingCopy => Checkout::StayPut,
                        Checkout::StayPut => Checkout::MoveWorkingCopy,
                    }
                },
                is_on: |c| *c == Checkout::StayPut,
            }],
        },
        Group {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "o",
                    desc: "New child of the revision at point",
                    build: |c| TransientAction::New(Placement::ChildOf, *c),
                },
                Item::Action {
                    key: "a",
                    desc: "Insert after the revision at point",
                    build: |c| TransientAction::New(Placement::After, *c),
                },
                Item::Action {
                    key: "b",
                    desc: "Insert before the revision at point",
                    build: |c| TransientAction::New(Placement::Before, *c),
                },
            ],
        },
    ],
};

pub static SQUASH: MenuDef<SquashOptions> = MenuDef {
    title: "Squash",
    groups: &[
        Group {
            title: "Arguments",
            items: &[
                Item::Switch {
                    key: "-k",
                    label: "--keep-emptied",
                    desc: "Keep the emptied source revision",
                    toggle: |o| {
                        o.emptied = match o.emptied {
                            EmptiedSource::Abandon => EmptiedSource::Keep,
                            EmptiedSource::Keep => EmptiedSource::Abandon,
                        }
                    },
                    is_on: |o| o.emptied == EmptiedSource::Keep,
                },
                Item::Switch {
                    key: "-u",
                    label: "--use-destination-message",
                    desc: "Keep the destination's description",
                    toggle: |o| {
                        o.description = match o.description {
                            SquashDescription::Combine => SquashDescription::UseDestination,
                            SquashDescription::UseDestination => SquashDescription::Combine,
                        }
                    },
                    is_on: |o| o.description == SquashDescription::UseDestination,
                },
            ],
        },
        Group {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "s",
                    desc: "Squash the revision at point into its parent",
                    build: |o| TransientAction::Squash(o.clone()),
                },
                Item::Action {
                    key: "t",
                    desc: "Squash the working copy into the revision at point",
                    build: |o| TransientAction::SquashInto(o.clone()),
                },
            ],
        },
    ],
};

pub static REBASE: MenuDef<EmptiedCommits> = MenuDef {
    title: "Rebase",
    groups: &[
        Group {
            title: "Arguments",
            items: &[Item::Switch {
                key: "-e",
                label: "--skip-emptied",
                desc: "Abandon revisions that become empty",
                toggle: |e| {
                    *e = match e {
                        EmptiedCommits::Keep => EmptiedCommits::Skip,
                        EmptiedCommits::Skip => EmptiedCommits::Keep,
                    }
                },
                is_on: |e| *e == EmptiedCommits::Skip,
            }],
        },
        Group {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "r",
                    desc: "Rebase the revision at point",
                    build: |e| TransientAction::Rebase(RebaseKind::Revision, *e),
                },
                Item::Action {
                    key: "s",
                    desc: "Rebase the revision and its descendants",
                    build: |e| TransientAction::Rebase(RebaseKind::WithDescendants, *e),
                },
                Item::Action {
                    key: "b",
                    desc: "Rebase the whole branch",
                    build: |e| TransientAction::Rebase(RebaseKind::Branch, *e),
                },
            ],
        },
    ],
};

pub static BOOKMARK: MenuDef<()> = MenuDef {
    title: "Bookmark",
    groups: &[Group {
        title: "Actions",
        items: &[
            Item::Action {
                key: "c",
                desc: "Create at the revision at point",
                build: |_| TransientAction::BookmarkCreate,
            },
            Item::Action {
                key: "s",
                desc: "Set (move) to the revision at point",
                build: |_| TransientAction::BookmarkSet,
            },
            Item::Action {
                key: "d",
                desc: "Delete",
                build: |_| TransientAction::BookmarkDelete,
            },
            Item::Action {
                key: "r",
                desc: "Rename",
                build: |_| TransientAction::BookmarkRename,
            },
            Item::Action {
                key: "t",
                desc: "Track a remote bookmark",
                build: |_| TransientAction::BookmarkTrack,
            },
            Item::Action {
                key: "u",
                desc: "Untrack a remote bookmark",
                build: |_| TransientAction::BookmarkUntrack,
            },
            Item::Action {
                key: "f",
                desc: "Forget (local and remote refs)",
                build: |_| TransientAction::BookmarkForget,
            },
        ],
    }],
};

pub static PUSH: MenuDef<PushOptions> = MenuDef {
    title: "Push",
    groups: &[
        Group {
            title: "Arguments",
            items: &[
                Item::Switch {
                    key: "-d",
                    label: "--deleted",
                    desc: "Push deleted bookmarks",
                    toggle: |o| {
                        o.deletions = match o.deletions {
                            Deletions::Skip => Deletions::Propagate,
                            Deletions::Propagate => Deletions::Skip,
                        }
                    },
                    is_on: |o| o.deletions == Deletions::Propagate,
                },
                Item::Switch {
                    key: "-n",
                    label: "--dry-run",
                    desc: "Only show what would change",
                    toggle: |o| {
                        o.mode = match o.mode {
                            RunMode::Apply => RunMode::DryRun,
                            RunMode::DryRun => RunMode::Apply,
                        }
                    },
                    is_on: |o| o.mode == RunMode::DryRun,
                },
            ],
        },
        Group {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "p",
                    desc: "Push all tracked bookmarks",
                    build: |o| TransientAction::Push(o.clone()),
                },
                Item::Action {
                    key: "b",
                    desc: "Push a bookmark",
                    build: |o| TransientAction::PushBookmark(o.clone()),
                },
                Item::Action {
                    key: "c",
                    desc: "Push the change at point (creates a bookmark)",
                    build: |o| TransientAction::PushChange(o.clone()),
                },
            ],
        },
    ],
};

pub static FETCH: MenuDef<()> = MenuDef {
    title: "Fetch",
    groups: &[Group {
        title: "Actions",
        items: &[
            Item::Action {
                key: "f",
                desc: "Fetch from the default remote",
                build: |_| TransientAction::Fetch(Fetch::DefaultRemote),
            },
            Item::Action {
                key: "a",
                desc: "Fetch from all remotes",
                build: |_| TransientAction::Fetch(Fetch::AllRemotes),
            },
        ],
    }],
};

/// The log menu's choices: an optional row limit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogChoices {
    pub limit: Option<NonZeroUsize>,
}

pub static LOG: MenuDef<LogChoices> = MenuDef {
    title: "Log",
    groups: &[
        Group {
            title: "Arguments",
            items: &[Item::Arg {
                key: "-n",
                label: "-n",
                desc: "Limit number of revisions",
                set: |c, text| {
                    if text.is_empty() {
                        c.limit = None;
                        return Ok(());
                    }
                    c.limit = Some(
                        text.parse()
                            .map_err(|_| "limit must be a positive number".to_string())?,
                    );
                    Ok(())
                },
                shown: |c| c.limit.map(|n| n.to_string()),
            }],
        },
        Group {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "l",
                    desc: "Log the default revset",
                    build: |c| TransientAction::LogDefault(c.limit),
                },
                Item::Action {
                    key: "a",
                    desc: "Log all revisions",
                    build: |c| TransientAction::LogAll(c.limit),
                },
                Item::Action {
                    key: "o",
                    desc: "Log another revset",
                    build: |c| TransientAction::LogRevset(c.limit),
                },
            ],
        },
    ],
};

/// Parse an `Arg` value into an optional choice; empty clears it.
fn optional<T>(text: &str, parse: fn(&str) -> Result<T, Invalid>) -> Result<Option<T>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    parse(text).map(Some).map_err(|e| e.to_string())
}

/// The restore menu's choices. `--changes-in` is an action, not an
/// argument, since it takes the revision at point.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreChoices {
    pub from: Option<Revset>,
    pub into: Option<Revset>,
    pub fileset: Option<Fileset>,
    pub descendants: RestoreDescendants,
    pub interactive: bool,
    /// Implies interactive.
    pub tool: Option<DiffTool>,
}

pub static RESTORE: MenuDef<RestoreChoices> = MenuDef {
    title: "Restore",
    groups: &[
        Group {
            title: "Arguments",
            items: &[
                Item::Arg {
                    key: "-f",
                    label: "--from",
                    desc: "Revision to restore from",
                    set: |c, t| {
                        c.from = optional(t, Revset::parse)?;
                        Ok(())
                    },
                    shown: |c| c.from.as_ref().map(ToString::to_string),
                },
                Item::Arg {
                    key: "-t",
                    label: "--into",
                    desc: "Revision to restore into",
                    set: |c, t| {
                        c.into = optional(t, Revset::parse)?;
                        Ok(())
                    },
                    shown: |c| c.into.as_ref().map(ToString::to_string),
                },
                Item::Switch {
                    key: "-d",
                    label: "--restore-descendants",
                    desc: "Keep descendants' content, not their diffs",
                    toggle: |c| {
                        c.descendants = match c.descendants {
                            RestoreDescendants::KeepDiffs => RestoreDescendants::KeepContents,
                            RestoreDescendants::KeepContents => RestoreDescendants::KeepDiffs,
                        }
                    },
                    is_on: |c| c.descendants == RestoreDescendants::KeepContents,
                },
                Item::Switch {
                    key: "-i",
                    label: "--interactive",
                    desc: "Choose the changes in the diff editor",
                    toggle: |c| c.interactive = !c.interactive,
                    is_on: |c| c.interactive,
                },
                Item::Arg {
                    key: "=t",
                    label: "--tool",
                    desc: "Diff editor to use (implies --interactive)",
                    set: |c, t| {
                        c.tool = optional(t, DiffTool::parse)?;
                        Ok(())
                    },
                    shown: |c| c.tool.as_ref().map(ToString::to_string),
                },
                Item::Arg {
                    key: "--",
                    label: "--",
                    desc: "Restore only these paths (fileset)",
                    set: |c, t| {
                        c.fileset = optional(t, Fileset::parse)?;
                        Ok(())
                    },
                    shown: |c| c.fileset.as_ref().map(ToString::to_string),
                },
            ],
        },
        Group {
            title: "Actions",
            items: &[
                Item::Action {
                    key: "r",
                    desc: "Restore (default: @ from its parent)",
                    build: |c| TransientAction::Restore(c.clone()),
                },
                Item::Action {
                    key: "c",
                    desc: "Undo the changes in the revision at point",
                    build: |c| TransientAction::RestoreChangesIn(c.clone()),
                },
            ],
        },
    ],
};

/// Open a `Command::Transient(menu)`. A new menu adds one arm here and
/// nothing in `App::dispatch`.
pub fn open(menu: Menu) -> Box<dyn OpenMenu> {
    match menu {
        Menu::Describe => Box::new(TransientState::new(&DESCRIBE)),
        Menu::Commit => Box::new(TransientState::new(&COMMIT)),
        Menu::New => Box::new(TransientState::new(&NEW)),
        Menu::Squash => Box::new(TransientState::new(&SQUASH)),
        Menu::Rebase => Box::new(TransientState::new(&REBASE)),
        Menu::Bookmark => Box::new(TransientState::new(&BOOKMARK)),
        Menu::Push => Box::new(TransientState::new(&PUSH)),
        Menu::Fetch => Box::new(TransientState::new(&FETCH)),
        Menu::Log => Box::new(TransientState::new(&LOG)),
        Menu::Restore => Box::new(TransientState::new(&RESTORE)),
    }
}

/// Outcome of feeding one key to an open transient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransientResult {
    /// Key consumed (switch toggled or prefix pending); keep the menu open.
    Consumed,
    /// Prompt for the value of the `Arg` item with this key; the menu stays
    /// open and the answer comes back through `OpenMenu::set_value`.
    Prompt {
        key: &'static str,
        desc: &'static str,
    },
    /// Run this action; menu closes.
    Invoke(TransientAction),
    /// Close without running anything.
    Cancel,
    /// Key didn't match any item.
    Unbound,
}

/// An open menu with its choice type erased, so `App` can hold any menu.
pub trait OpenMenu {
    fn title(&self) -> &'static str;
    fn on_key(&mut self, kp: &KeyPress) -> TransientResult;
    /// Answer a `Prompt` for the `Arg` with `key`. `Err` is a message for
    /// the user; the previous value stays.
    fn set_value(&mut self, key: &str, text: &str) -> Result<(), String>;
    /// Lines for the bottom panel.
    fn render_lines(&self, t: &Theme) -> Vec<Line<'static>>;
}

/// A currently-open transient: the definition, the choices made so far and
/// the multi-char key input buffer (switch keys like "-a" are two keystrokes).
pub struct TransientState<S: 'static> {
    def: &'static MenuDef<S>,
    pub choices: S,
    pending: String,
}

impl<S: Default + 'static> TransientState<S> {
    pub fn new(def: &'static MenuDef<S>) -> Self {
        Self {
            def,
            choices: S::default(),
            pending: String::new(),
        }
    }
}

impl<S: 'static> TransientState<S> {
    fn items(&self) -> impl Iterator<Item = &'static Item<S>> {
        self.def.groups.iter().flat_map(|g| g.items.iter())
    }
}

impl<S: 'static> OpenMenu for TransientState<S> {
    fn title(&self) -> &'static str {
        self.def.title
    }

    fn on_key(&mut self, kp: &KeyPress) -> TransientResult {
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

        if let Some(item) = self.items().find(|i| i.key() == self.pending) {
            self.pending.clear();
            match item {
                Item::Switch { toggle, .. } => {
                    toggle(&mut self.choices);
                    TransientResult::Consumed
                }
                Item::Arg { key, desc, .. } => TransientResult::Prompt { key, desc },
                Item::Action { build, .. } => TransientResult::Invoke(build(&self.choices)),
            }
        } else if self.items().any(|i| i.key().starts_with(&self.pending)) {
            TransientResult::Consumed
        } else {
            self.pending.clear();
            TransientResult::Unbound
        }
    }

    fn set_value(&mut self, key: &str, text: &str) -> Result<(), String> {
        match self.items().find(|i| i.key() == key) {
            Some(Item::Arg { set, .. }) => set(&mut self.choices, text.trim()),
            _ => Err(format!("{key} takes no value")),
        }
    }

    fn render_lines(&self, t: &Theme) -> Vec<Line<'static>> {
        let mut out = Vec::new();
        for group in self.def.groups {
            out.push(Line::from(Span::styled(
                group.title.to_string(),
                Style::new().fg(t.menu_title).add_modifier(Modifier::BOLD),
            )));
            for item in group.items {
                match item {
                    Item::Switch {
                        key,
                        label,
                        desc,
                        is_on,
                        ..
                    } => {
                        let style = if is_on(&self.choices) {
                            Style::new().fg(t.transient_argument).bold()
                        } else {
                            Style::new().dim()
                        };
                        out.push(Line::from(vec![
                            Span::raw(" "),
                            Span::styled(format!("{key:<4}"), Style::new().fg(t.key)),
                            Span::styled(format!("{label:<26}"), style),
                            Span::raw(desc.to_string()),
                        ]));
                    }
                    Item::Arg {
                        key,
                        label,
                        desc,
                        shown,
                        ..
                    } => {
                        let (text, style) = match shown(&self.choices) {
                            Some(v) => (
                                format!("{label} {v}"),
                                Style::new().fg(t.transient_argument).bold(),
                            ),
                            None => (label.to_string(), Style::new().dim()),
                        };
                        out.push(Line::from(vec![
                            Span::raw(" "),
                            Span::styled(format!("{key:<4}"), Style::new().fg(t.key)),
                            Span::styled(format!("{text:<26}"), style),
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
        assert_eq!(st.choices.emptied, EmptiedSource::Keep);
        // Toggling off again.
        st.on_key(&key('-'));
        st.on_key(&key('k'));
        assert_eq!(st.choices, SquashOptions::default());
    }

    #[test]
    fn action_carries_the_choices_made() {
        let mut st = TransientState::new(&SQUASH);
        st.on_key(&key('-'));
        st.on_key(&key('u'));
        assert_eq!(
            st.on_key(&key('s')),
            TransientResult::Invoke(TransientAction::Squash(SquashOptions {
                emptied: EmptiedSource::Abandon,
                description: SquashDescription::UseDestination,
            }))
        );
    }

    #[test]
    fn value_arg_prompts_then_parses_into_the_choice() {
        let mut st = TransientState::new(&LOG);
        assert_eq!(st.on_key(&key('-')), TransientResult::Consumed);
        assert_eq!(
            st.on_key(&key('n')),
            TransientResult::Prompt {
                key: "-n",
                desc: "Limit number of revisions",
            }
        );
        st.set_value("-n", "50").unwrap();
        assert_eq!(
            st.on_key(&key('l')),
            TransientResult::Invoke(TransientAction::LogDefault(NonZeroUsize::new(50)))
        );
    }

    #[test]
    fn bad_value_is_rejected_and_empty_clears() {
        let mut st = TransientState::new(&LOG);
        st.set_value("-n", "50").unwrap();
        assert!(st.set_value("-n", "lots").is_err());
        assert!(st.set_value("-n", "0").is_err());
        assert_eq!(st.choices.limit, NonZeroUsize::new(50));
        st.set_value("-n", "").unwrap();
        assert_eq!(st.choices.limit, None);
    }

    #[test]
    fn switch_and_action_sharing_a_letter_stay_distinct() {
        // NEW has both the "-E" switch prefix "-" and actions; the pending
        // prefix must keep two-key sequences separate from actions.
        let mut st = TransientState::new(&NEW);
        st.on_key(&key('-'));
        st.on_key(&key('E'));
        assert_eq!(st.choices, Checkout::StayPut);
        assert_eq!(
            st.on_key(&key('b')),
            TransientResult::Invoke(TransientAction::New(Placement::Before, Checkout::StayPut))
        );
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
