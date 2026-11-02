//! Names this build uses on disk, in the environment, and with the OS.
//!
//! The binary target picks the identity. `herdr` keeps upstream's names.
//! `herdr-fork` (`cargo build --release --features fork --bin herdr-fork`)
//! gets its own, so it runs beside an installed herdr without sharing config,
//! state, sockets, sessions, launchd jobs, or worktrees with it.

pub const APP_NAME: &str = match option_env!("CARGO_BIN_NAME") {
    Some(name) => name,
    None => "herdr",
};

pub const IS_FORK: bool = str_eq(APP_NAME, "herdr-fork");

pub const DEV_APP_DIR_NAME: &str = if IS_FORK {
    "herdr-fork-dev"
} else {
    "herdr-dev"
};

pub const ENV_MARKER_VAR: &str = if IS_FORK {
    "HERDR_FORK_ENV"
} else {
    "HERDR_ENV"
};

pub const SOCKET_PATH_ENV_VAR: &str = if IS_FORK {
    "HERDR_FORK_SOCKET_PATH"
} else {
    "HERDR_SOCKET_PATH"
};

pub const CLIENT_SOCKET_PATH_ENV_VAR: &str = if IS_FORK {
    "HERDR_FORK_CLIENT_SOCKET_PATH"
} else {
    "HERDR_CLIENT_SOCKET_PATH"
};

pub const CONFIG_PATH_ENV_VAR: &str = if IS_FORK {
    "HERDR_FORK_CONFIG_PATH"
} else {
    "HERDR_CONFIG_PATH"
};

pub const SESSION_ENV_VAR: &str = if IS_FORK {
    "HERDR_FORK_SESSION"
} else {
    "HERDR_SESSION"
};

pub const DEFAULT_WORKTREES_DIR: &str = if IS_FORK {
    "~/.herdr-fork/worktrees"
} else {
    "~/.herdr/worktrees"
};

/// Agent integration hooks read these upstream names, so herdr-fork panes
/// export them too and hooks installed by herdr report to herdr-fork.
pub const SHARED_HOOK_ENV_MARKER_VAR: &str = "HERDR_ENV";
pub const SHARED_HOOK_SOCKET_PATH_ENV_VAR: &str = "HERDR_SOCKET_PATH";

const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut index = 0;
    while index < a.len() {
        if a[index] != b[index] {
            return false;
        }
        index += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn herdr_binary_keeps_upstream_names() {
        assert_eq!(APP_NAME, "herdr");
        assert_eq!(DEV_APP_DIR_NAME, "herdr-dev");
        assert_eq!(ENV_MARKER_VAR, "HERDR_ENV");
        assert_eq!(SOCKET_PATH_ENV_VAR, "HERDR_SOCKET_PATH");
        assert_eq!(CLIENT_SOCKET_PATH_ENV_VAR, "HERDR_CLIENT_SOCKET_PATH");
        assert_eq!(CONFIG_PATH_ENV_VAR, "HERDR_CONFIG_PATH");
        assert_eq!(SESSION_ENV_VAR, "HERDR_SESSION");
        assert_eq!(DEFAULT_WORKTREES_DIR, "~/.herdr/worktrees");
    }

    #[test]
    fn str_eq_compares_whole_strings() {
        assert!(str_eq("herdr-fork", "herdr-fork"));
        assert!(!str_eq("herdr", "herdr-fork"));
        assert!(!str_eq("herdr-forx", "herdr-fork"));
    }
}
