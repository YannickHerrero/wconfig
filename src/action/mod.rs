pub mod launch;
pub mod url;

use anyhow::Result;

use crate::config::Action;

pub fn run(action: &Action) -> Result<()> {
    match action {
        Action::Launch { command } => launch::run(command),
        Action::Url { url } => url::open(url),
        Action::Script { .. } => anyhow::bail!("script action not yet implemented"),
        Action::FocusOrLaunch { .. } => anyhow::bail!("focus_or_launch not yet implemented"),
    }
}
