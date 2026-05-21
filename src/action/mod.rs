pub mod launch;

use anyhow::Result;

use crate::config::Action;

pub fn run(action: &Action) -> Result<()> {
    match action {
        Action::Launch { command } => launch::run(command),
        Action::Url { .. } => anyhow::bail!("url action not yet implemented"),
        Action::Script { .. } => anyhow::bail!("script action not yet implemented"),
        Action::FocusOrLaunch { .. } => anyhow::bail!("focus_or_launch not yet implemented"),
    }
}
