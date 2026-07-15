//! Git-interop actions: `jj git fetch` and `jj git push`.

use crate::ui::transient::TransientAction;

use super::super::{App, Input};

impl App {
    pub(super) fn remote_action(&mut self, action: TransientAction, args: Vec<String>) {
        match action {
            TransientAction::Fetch => {
                self.run_jj_bg("fetch".into(), vec!["git".into(), "fetch".into()]);
            }
            TransientAction::FetchAllRemotes => {
                self.run_jj_bg(
                    "fetch all remotes".into(),
                    vec!["git".into(), "fetch".into(), "--all-remotes".into()],
                );
            }
            TransientAction::Push => {
                let mut full = vec!["git".to_string(), "push".to_string()];
                full.extend(args);
                self.run_jj_bg("push".into(), full);
            }
            TransientAction::PushBookmark => {
                let candidates: Vec<String> = self
                    .snapshot
                    .as_ref()
                    .map(|s| {
                        s.bookmarks
                            .iter()
                            .filter(|b| b.remote.is_none())
                            .map(|b| b.name.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                if candidates.is_empty() {
                    self.message = Some("no local bookmarks".into());
                } else {
                    self.input = Some(Input::picker(
                        "Push bookmark",
                        candidates,
                        move |app, bookmark| {
                            let mut full = vec![
                                "git".to_string(),
                                "push".to_string(),
                                "--bookmark".to_string(),
                                bookmark.clone(),
                            ];
                            full.extend(args);
                            app.run_jj_bg(format!("push {bookmark}"), full);
                        },
                    ));
                }
            }
            TransientAction::PushChange => {
                let rev = self.rev_at_point();
                let mut full = vec![
                    "git".to_string(),
                    "push".to_string(),
                    "--change".to_string(),
                    rev.clone(),
                ];
                full.extend(args);
                self.run_jj_bg(format!("push change {rev}"), full);
            }
            _ => unreachable!("routed by invoke_transient"),
        }
    }
}
