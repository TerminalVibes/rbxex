use std::path::PathBuf;

pub(crate) fn resolve_command(command: &str) -> Option<PathBuf> {
    which::which_global(command).ok()
}
