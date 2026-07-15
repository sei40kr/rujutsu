//! Rebase actions: pick a destination, then `jj rebase -r/-s/-b SRC -d DEST`.

use crate::jj::client::JjClient;
use crate::ui::transient::TransientAction;

use super::super::{App, Input};

impl App {
    pub(super) fn rebase_action(&mut self, action: TransientAction, switches: Vec<String>) {
        let mode = match action {
            TransientAction::RebaseRev => "-r",
            TransientAction::RebaseSource => "-s",
            TransientAction::RebaseBranch => "-b",
            _ => unreachable!("routed by invoke_transient"),
        };
        let rev = self.rev_at_point();
        let candidates = JjClient::rev_candidates(self.snapshot.as_ref());
        // The destination prompt closes over the mode, source and switches.
        self.input = Some(Input::picker(
            format!("Rebase {mode} {rev} onto"),
            candidates,
            move |app, dest| {
                let mut args = vec![
                    "rebase".to_string(),
                    mode.to_string(),
                    rev.clone(),
                    "-d".to_string(),
                    dest.clone(),
                ];
                args.extend(switches);
                app.run_jj_bg(format!("rebase {rev} onto {dest}"), args);
            },
        ));
    }
}
