//! Starting external programs (the assistant, lint and format tools, opening files).
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

/// A `Command` that never opens a console window on Windows.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// True inside a Flatpak sandbox.
pub fn in_flatpak() -> bool {
    Path::new("/.flatpak-info").is_file()
}

/// A program installed by the user on their system (the assistant's `claude`, `codex`,
/// or `ollama`, or a lint tool), run in `cwd`.
///
/// Inside a Flatpak those programs are outside the sandbox, so they are started on the
/// host with `flatpak-spawn --host` (this needs the `org.freedesktop.Flatpak` bus
/// permission). `--watch-bus` ends the host program when `flatpak-spawn` is killed, so
/// cancelling still works. Elsewhere (including a classic snap) this is `command`.
pub fn user_program(program: impl AsRef<OsStr>, cwd: &Path) -> Command {
    let mut command = if in_flatpak() {
        let mut spawn = command("flatpak-spawn");
        spawn.arg("--host").arg("--watch-bus");
        if cwd.is_dir() {
            spawn.arg(format!("--directory={}", cwd.display()));
        }
        spawn.arg(program);
        spawn
    } else {
        command(program)
    };
    if cwd.is_dir() {
        command.current_dir(cwd);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_programs_run_in_the_folder() {
        let temp = tempfile::tempdir().unwrap();
        let command = user_program("claude", temp.path());
        assert_eq!(command.get_current_dir(), Some(temp.path()));
        if in_flatpak() {
            assert_eq!(command.get_program(), "flatpak-spawn");
        } else {
            assert_eq!(command.get_program(), "claude");
        }
    }
}
