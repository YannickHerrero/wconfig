pub mod focus_or_launch;
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
        Action::FocusOrLaunch {
            exe_path,
            match_basename,
            launch_args,
        } => focus_or_launch::run(exe_path, *match_basename, launch_args),
    }
}
