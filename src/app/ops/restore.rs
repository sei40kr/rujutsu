//! Restore actions: restore between two revisions, or undo the changes in
//! the revision at point. Interactive restores hand the terminal to jj's
//! diff editor; the rest run in the background.

use crate::jj::op::{Restore, RestoreSource, RestoreSpec};
use crate::ui::transient::{RestoreChoices, TransientAction};

use super::super::App;

impl App {
    pub(super) fn restore_action(&mut self, action: TransientAction) {
        let (choices, source) = match action {
            TransientAction::Restore(c) => {
                let source = RestoreSource::Between {
                    from: c.from.clone(),
                    into: c.into.clone(),
                };
                (c, source)
            }
            TransientAction::RestoreChangesIn(c) => {
                if c.from.is_some() || c.into.is_some() {
                    self.message =
                        Some("--changes-in cannot be combined with --from/--into".into());
                    return;
                }
                (c, RestoreSource::ChangesIn(self.rev_at_point()))
            }
            _ => unreachable!("routed by invoke_transient"),
        };
        let RestoreChoices {
            fileset,
            descendants,
            interactive,
            tool,
            ..
        } = choices;
        let desc = match &source {
            RestoreSource::ChangesIn(r) => format!("restore changes in {r}"),
            RestoreSource::Between { from, into } => format!(
                "restore {} from {}",
                into.as_ref().map_or("@".into(), ToString::to_string),
                from.as_ref()
                    .map_or("its parent".into(), ToString::to_string),
            ),
        };
        let op = Restore::Spec(RestoreSpec {
            source,
            fileset,
            descendants,
        });
        if interactive || tool.is_some() {
            self.request_editor(desc, move |jj| jj.restore_in_editor(&op, tool.as_ref()));
        } else {
            self.run_bg(desc, move |jj| jj.restore(&op));
        }
    }
}
