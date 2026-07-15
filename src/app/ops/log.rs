//! Log actions: open log buffers over various revsets.

use crate::jj::client::JjClient;
use crate::ui::transient::TransientAction;

use super::super::{App, Input};

impl App {
    pub(super) fn log_action(&mut self, action: TransientAction, args: Vec<String>) {
        match action {
            TransientAction::LogDefault => {
                let mut full = vec!["log".to_string()];
                if let Some(revset) = &self.jj.log_revset {
                    full.extend(["-r".to_string(), revset.clone()]);
                }
                full.extend(args);
                self.load_log("Log".into(), full, false);
            }
            TransientAction::LogAll => {
                let mut full = vec!["log".to_string(), "-r".to_string(), "::".to_string()];
                full.extend(args);
                self.load_log("Log ::".into(), full, false);
            }
            TransientAction::LogRevset => {
                let candidates = JjClient::rev_candidates(self.snapshot.as_ref());
                self.input = Some(Input::picker(
                    "Log revset",
                    candidates,
                    move |app, revset| {
                        let mut full = vec!["log".to_string(), "-r".to_string(), revset.clone()];
                        full.extend(args);
                        app.load_log(format!("Log {revset}"), full, false);
                    },
                ));
            }
            _ => unreachable!("routed by invoke_transient"),
        }
    }
}
