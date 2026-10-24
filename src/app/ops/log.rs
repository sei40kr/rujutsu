//! Log actions: open log buffers over various revsets.

use std::num::NonZeroUsize;

use crate::jj::op::{LogQuery, LogScope, Revset};
use crate::jj::types::rev_candidates;
use crate::ui::transient::TransientAction;

use super::super::{App, Input};

impl App {
    pub(super) fn log_action(&mut self, action: TransientAction) {
        match action {
            TransientAction::LogDefault(limit) => {
                let scope = match &self.log_revset {
                    Some(r) => LogScope::Revset(r.clone()),
                    None => LogScope::Default,
                };
                self.load_log("Log".into(), LogQuery { scope, limit }, false);
            }
            TransientAction::LogAll(limit) => {
                let scope = LogScope::Revset(Revset::parse("::").expect("literal revset"));
                self.load_log("Log ::".into(), LogQuery { scope, limit }, false);
            }
            TransientAction::LogRevset(limit) => self.prompt_log_revset(limit),
            _ => unreachable!("routed by invoke_transient"),
        }
    }

    fn prompt_log_revset(&mut self, limit: Option<NonZeroUsize>) {
        let candidates = rev_candidates(self.snapshot.as_ref())
            .iter()
            .map(ToString::to_string)
            .collect();
        self.input = Some(Input::picker("Log revset", candidates, move |app, text| {
            let Some(revset) = app.accept(Revset::parse(&text)) else {
                return;
            };
            let title = format!("Log {revset}");
            let query = LogQuery {
                scope: LogScope::Revset(revset),
                limit,
            };
            app.load_log(title, query, false);
        }));
    }
}
