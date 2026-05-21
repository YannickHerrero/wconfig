pub mod launch;
pub mod script;
pub mod url;

use anyhow::Result;

use crate::config::Action;

pub fn run(action: &Action) -> Result<()> {
    match action {
        Action::Launch { command } => launch::run(command),
        Action::Url { url } => url::open(url),
        Action::Script { shell, script } => script::run(*shell, script),
        Action::FocusOrLaunch { .. } => anyhow::bail!("focus_or_launch not yet implemented"),
    }
}
