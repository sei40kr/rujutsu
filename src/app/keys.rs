//! Key routing: the confirm > input > help > transient > keymap priority
//! chain.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::keymap::{normalize, KeyPress, Lookup, PaneKind};
use crate::ui::input::InputResult;
use crate::ui::transient::TransientResult;

use super::{App, Input, PendingAction};

impl App {
    pub(super) fn on_key(&mut self, ev: KeyEvent) {
        if ev.kind != KeyEventKind::Press {
            return;
        }
        let kp = normalize(&ev);
        self.message = None;

        if self.confirm.is_some() {
            self.on_confirm_key(&kp);
            return;
        }
        if let Some(mut input) = self.input.take() {
            // Take the input out so the callbacks can borrow `self` freely;
            // a `Consumed` edit restores it, and `on_submit` is free to open a
            // follow-up prompt (e.g. bookmark rename's second step).
            match input.state.on_key(&kp) {
                InputResult::Consumed => {
                    input.changed(self);
                    self.input = Some(input);
                }
                InputResult::Cancel => {
                    input.cancelled(self);
                    self.message = Some("aborted".into());
                }
                InputResult::Submit(value) => {
                    input.submit(self, value);
                }
            }
            return;
        }
        if self.show_help {
            let ctrl = kp.mods.contains(KeyModifiers::CONTROL);
            match kp.code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => self.show_help = false,
                KeyCode::Char('j') | KeyCode::Down => self.help_scroll += 1,
                KeyCode::Char('k') | KeyCode::Up => {
                    self.help_scroll = self.help_scroll.saturating_sub(1)
                }
                KeyCode::Char('d') if ctrl => self.help_scroll += 10,
                KeyCode::Char('u') if ctrl => {
                    self.help_scroll = self.help_scroll.saturating_sub(10)
                }
                KeyCode::PageDown => self.help_scroll += 10,
                KeyCode::PageUp => self.help_scroll = self.help_scroll.saturating_sub(10),
                KeyCode::Home | KeyCode::Char('g') => self.help_scroll = 0,
                // Clamped down to the last line by render.
                KeyCode::End | KeyCode::Char('G') => self.help_scroll = usize::MAX,
                _ => {}
            }
            return;
        }
        if let Some(transient) = self.transient.as_mut() {
            match transient.on_key(&kp) {
                TransientResult::Consumed => {}
                TransientResult::Cancel => self.transient = None,
                TransientResult::Unbound => {
                    self.message = Some("key not bound in this menu".into());
                }
                TransientResult::Prompt { flag, desc } => {
                    // Prompt for the value over the still-open transient; the
                    // submit callback writes it back into `transient.values`.
                    self.input = Some(Input::plain(desc, move |app, value| {
                        if let Some(t) = app.transient.as_mut() {
                            t.set_value(flag, value);
                        }
                    }));
                }
                TransientResult::Invoke(action, args) => {
                    self.transient = None;
                    self.invoke_transient(action, args);
                }
            }
            return;
        }

        if kp.code == KeyCode::Esc {
            if !self.pending.is_empty() {
                self.pending.clear();
            } else if self.search.query.take().is_some() {
                self.message = Some("search cleared".into());
            }
            return;
        }
        self.pending.push(kp);
        let kind = self
            .panes
            .last()
            .map(|p| p.kind)
            .unwrap_or(PaneKind::Status);
        match self.keymaps.lookup(kind, &self.pending) {
            Lookup::Command(cmd) => {
                self.pending.clear();
                self.dispatch(cmd);
            }
            Lookup::Pending => {}
            Lookup::Unbound => {
                if self.pending.len() > 1 {
                    self.message = Some(format!(
                        "{} is undefined",
                        crate::keymap::format_keys(&self.pending)
                    ));
                }
                self.pending.clear();
            }
        }
    }

    fn on_confirm_key(&mut self, kp: &KeyPress) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        if matches!(
            kp.code,
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter
        ) {
            match confirm.action {
                PendingAction::Jj { desc, args } => self.run_jj_bg(desc, args),
            }
        } else {
            self.message = Some("aborted".into());
        }
    }
}
