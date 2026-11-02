# herdr-fork

This fork builds a second binary, `herdr-fork`, that runs next to an installed
upstream `herdr` without sharing anything with it. The `herdr` binary built
from this repository is unchanged upstream herdr.

## Build

```sh
cargo build --release --locked --features fork --bin herdr-fork
```

The result is `target/release/herdr-fork`. Building needs the repository's
Rust toolchain (rustup picks it up from `rust-toolchain.toml`) and Zig 0.16.0,
either on `PATH` or named by `ZIG`.

## Install on macOS

These steps leave the installed `herdr` binary, its config, and its running
sessions alone. Everything lives under `~/.herdr-fork` and is appended to
`PATH`, so no existing command is shadowed.

```sh
git clone https://github.com/felarof99/herdr.git ~/.herdr-fork/src
cd ~/.herdr-fork/src

arch=$(uname -m | sed 's/arm64/aarch64/')
mkdir -p ~/.herdr-fork/zig
curl -fsSL "https://ziglang.org/download/0.16.0/zig-$arch-macos-0.16.0.tar.xz" \
  | tar -xJ -C ~/.herdr-fork/zig --strip-components 1

ZIG=~/.herdr-fork/zig/zig cargo build --release --locked --features fork --bin herdr-fork
mkdir -p ~/.herdr-fork/bin
install -m 755 target/release/herdr-fork ~/.herdr-fork/bin/herdr-fork

echo 'export PATH="$PATH:$HOME/.herdr-fork/bin"' >> ~/.zshrc
exec zsh
herdr-fork --version
```

To update, `git pull` in `~/.herdr-fork/src`, rebuild, and reinstall the
binary. `herdr-fork update` refuses on purpose, because upstream's updater
would replace it with stock herdr.

Optional zsh completions: `herdr-fork completion zsh > "${fpath[1]}/_herdr-fork"`.

To remove it, run `herdr-fork server stop`, delete `~/.config/herdr-fork` and
`~/.local/state/herdr-fork`, and delete `~/.herdr-fork` once nothing in its
`worktrees` folder is still needed.

## What is separate

`src/app_identity.rs` holds every name below. The binary target picks the
set: `herdr` gets upstream's names and `herdr-fork` gets its own.

| Upstream herdr | herdr-fork |
| --- | --- |
| binary `herdr` | `herdr-fork` |
| `~/.config/herdr` (config, sessions, sockets, logs, plugins) | `~/.config/herdr-fork` |
| `~/.local/state/herdr` (machines, preferences, detection cache) | `~/.local/state/herdr-fork` |
| `HERDR_SOCKET_PATH`, `HERDR_CLIENT_SOCKET_PATH` | `HERDR_FORK_SOCKET_PATH`, `HERDR_FORK_CLIENT_SOCKET_PATH` |
| `HERDR_CONFIG_PATH`, `HERDR_SESSION` | `HERDR_FORK_CONFIG_PATH`, `HERDR_FORK_SESSION` |
| `HERDR_ENV` (nested-launch guard) | `HERDR_FORK_ENV` |
| launchd jobs `dev.herdr.server.*` | `dev.herdr-fork.server.*` |
| default worktrees `~/.herdr/worktrees` | `~/.herdr-fork/worktrees` |
| Kitty image temp dir `herdr-kitty-files-<uid>` | `herdr-fork-kitty-files-<uid>` |
| shell completions for `herdr` | for `herdr-fork` |
| self-update and background update checks | disabled |
| `integration install` and `uninstall` | refused |

Agent integrations are shared on purpose. Their hooks live in each agent's
config (`~/.claude`, `~/.codex`, and so on) and find herdr through the pane
environment, so herdr-fork panes also export `HERDR_ENV` and
`HERDR_SOCKET_PATH` pointing at herdr-fork. Install integrations with
`herdr integration install <agent>` and they work in both.

`scripts/herdr_fork_isolation_smoke.sh <herdr> <herdr-fork>` runs both side by
side in a throwaway `HOME` and checks all of this; the `herdr-fork` workflow
runs it on every pull request.
