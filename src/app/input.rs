//! The open minibuffer input, plus what to do with its result. The widget
//! (`state`, in `ui::input`) is app-agnostic; the callbacks here close over
//! the pending operation — the rebase flags awaiting a destination, the
//! revision a bookmark will point at, and so on — so there is no central
//! routing table and no opaque `carry` to unpack positionally.

use crate::ui::input::InputState;

use super::App;

/// Runs with the submitted (trimmed) text when the user hits RET.
type OnSubmit = Box<dyn FnOnce(&mut App, String)>;
/// Runs on every edit; the arg is the current text (for incremental search).
type OnChange = Box<dyn FnMut(&mut App, &str)>;
/// Runs when the user aborts with ESC / C-g.
type OnCancel = Box<dyn FnOnce(&mut App)>;

pub struct Input {
    pub(crate) state: InputState,
    on_submit: OnSubmit,
    on_change: Option<OnChange>,
    on_cancel: Option<OnCancel>,
    /// When false (the default), an empty submission is rejected with a
    /// message instead of firing `on_submit`.
    allow_empty: bool,
}

impl Input {
    /// A free-text prompt.
    pub(super) fn plain(
        prompt: impl Into<String>,
        on_submit: impl FnOnce(&mut App, String) + 'static,
    ) -> Self {
        Self::new(InputState::plain(prompt), on_submit)
    }

    /// A prompt that filters `candidates`; RET submits the selection (or the
    /// typed text when nothing matches).
    pub(super) fn picker(
        prompt: impl Into<String>,
        candidates: Vec<String>,
        on_submit: impl FnOnce(&mut App, String) + 'static,
    ) -> Self {
        Self::new(InputState::picker(prompt, candidates), on_submit)
    }

    fn new(state: InputState, on_submit: impl FnOnce(&mut App, String) + 'static) -> Self {
        Self {
            state,
            on_submit: Box::new(on_submit),
            on_change: None,
            on_cancel: None,
            allow_empty: false,
        }
    }

    /// React to every keystroke, not just the final submit.
    pub(super) fn on_change(mut self, f: impl FnMut(&mut App, &str) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    /// Run cleanup when the input is aborted.
    pub(super) fn on_cancel(mut self, f: impl FnOnce(&mut App) + 'static) -> Self {
        self.on_cancel = Some(Box::new(f));
        self
    }

    /// Let an empty submission through to `on_submit`.
    pub(super) fn allow_empty(mut self) -> Self {
        self.allow_empty = true;
        self
    }

    /// Feed the live text to `on_change`, if any.
    pub(super) fn changed(&mut self, app: &mut App) {
        if let Some(on_change) = self.on_change.as_mut() {
            let text = self.state.text.clone();
            on_change(app, &text);
        }
    }

    /// Fire `on_cancel`, if any.
    pub(super) fn cancelled(self, app: &mut App) {
        if let Some(on_cancel) = self.on_cancel {
            on_cancel(app);
        }
    }

    /// Fire `on_submit` with `value`, unless it is empty and empties are not
    /// allowed — in which case set a message and drop the input.
    pub(super) fn submit(self, app: &mut App, value: String) {
        if value.is_empty() && !self.allow_empty {
            app.message = Some("empty input".into());
            return;
        }
        (self.on_submit)(app, value);
    }
}
