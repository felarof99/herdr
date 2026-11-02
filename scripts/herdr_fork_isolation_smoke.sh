#!/usr/bin/env bash
# Runs herdr and herdr-fork side by side in a throwaway HOME and checks that
# neither binary reaches the other's server, sessions, or files.
#
# usage: scripts/herdr_fork_isolation_smoke.sh <herdr-binary> <herdr-fork-binary>
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <herdr-binary> <herdr-fork-binary>" >&2
  exit 2
fi

herdr=$(realpath "$1")
fork=$(realpath "$2")
home=$(mktemp -d)
herdr_pid=
fork_pid=

cleanup() {
  "$fork" server stop >/dev/null 2>&1 || true
  "$herdr" server stop >/dev/null 2>&1 || true
  for pid in "$fork_pid" "$herdr_pid"; do
    [ -n "$pid" ] && wait "$pid" 2>/dev/null || true
  done
  rm -rf "$home"
}
trap cleanup EXIT

while IFS= read -r name; do
  unset "$name"
done < <(env | sed -n -E 's/^((HERDR|XDG)_[A-Za-z0-9_]*)=.*/\1/p')
export HOME="$home"
cd "$home"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

pass() {
  echo "ok: $*"
}

server_socket() {
  "$1" status server | sed -n 's/^socket: //p'
}

wait_running() {
  for _ in $(seq 1 100); do
    if "$1" status server 2>/dev/null | grep -q '^status: running'; then
      return 0
    fi
    sleep 0.2
  done
  fail "$1 server did not start"
}

workspace_labels() {
  "$@" workspace list | python3 -c 'import json, sys; print("\n".join(w["label"] for w in json.load(sys.stdin)["result"]["workspaces"]))'
}

first_pane_of() {
  "$1" pane list | python3 -c 'import json, sys; ws = sys.argv[1]; print(next(p["pane_id"] for p in json.load(sys.stdin)["result"]["panes"] if p["workspace_id"] == ws))' "$2"
}

workspace_id_for() {
  "$1" workspace list | python3 -c 'import json, sys; label = sys.argv[1]; print(next(w["workspace_id"] for w in json.load(sys.stdin)["result"]["workspaces"] if w["label"] == label))' "$2"
}

echo "herdr:      $("$herdr" --version)"
echo "herdr-fork: $("$fork" --version)"
case "$("$fork" --version)" in
  herdr-fork\ *) pass "herdr-fork reports its own name" ;;
  *) fail "herdr-fork --version does not name herdr-fork" ;;
esac

"$herdr" server >"$home/herdr-server.log" 2>&1 &
herdr_pid=$!
"$fork" server >"$home/herdr-fork-server.log" 2>&1 &
fork_pid=$!
wait_running "$herdr"
wait_running "$fork"

herdr_socket=$(server_socket "$herdr")
fork_socket=$(server_socket "$fork")
echo "herdr socket:      $herdr_socket"
echo "herdr-fork socket: $fork_socket"
[ -S "$herdr_socket" ] || fail "herdr socket missing"
[ -S "$fork_socket" ] || fail "herdr-fork socket missing"
[ "$herdr_socket" != "$fork_socket" ] || fail "both servers use one socket"
[ "$(dirname "$herdr_socket")" != "$(dirname "$fork_socket")" ] || fail "both servers share a directory"
pass "servers listen on different sockets in different directories"

"$herdr" workspace create --cwd "$home" --label stock-only --no-focus >/dev/null
"$fork" workspace create --cwd "$home" --label fork-only --no-focus >/dev/null

herdr_labels=$(workspace_labels "$herdr")
fork_labels=$(workspace_labels "$fork")
grep -qx stock-only <<<"$herdr_labels" || fail "herdr lost its own workspace"
grep -qx fork-only <<<"$herdr_labels" && fail "herdr sees the herdr-fork workspace"
grep -qx fork-only <<<"$fork_labels" || fail "herdr-fork lost its own workspace"
grep -qx stock-only <<<"$fork_labels" && fail "herdr-fork sees the herdr workspace"
pass "each binary lists only its own workspaces"

herdr_pane=$(first_pane_of "$herdr" "$(workspace_id_for "$herdr" stock-only)")
fork_pane=$(first_pane_of "$fork" "$(workspace_id_for "$fork" fork-only)")

inherited_labels=$(HERDR_ENV=1 HERDR_SOCKET_PATH="$herdr_socket" HERDR_PANE_ID="$herdr_pane" HERDR_SESSION=default workspace_labels "$fork")
grep -qx fork-only <<<"$inherited_labels" || fail "herdr-fork followed herdr's pane environment"
grep -qx stock-only <<<"$inherited_labels" && fail "herdr-fork reached herdr's server from a herdr pane"
pass "herdr-fork ignores the environment of a herdr pane"

"$fork" pane run "$fork_pane" "env | grep -E '^HERDR' | sort > '$home/fork-pane.env'" >/dev/null
for _ in $(seq 1 50); do
  [ -s "$home/fork-pane.env" ] && break
  sleep 0.2
done
[ -s "$home/fork-pane.env" ] || fail "herdr-fork pane did not report its environment"
grep -qx "HERDR_FORK_SOCKET_PATH=$fork_socket" "$home/fork-pane.env" || fail "herdr-fork pane lacks HERDR_FORK_SOCKET_PATH"
grep -qx "HERDR_SOCKET_PATH=$fork_socket" "$home/fork-pane.env" || fail "herdr-fork pane hook socket does not point at herdr-fork"
grep -qx "HERDR_FORK_ENV=1" "$home/fork-pane.env" || fail "herdr-fork pane lacks HERDR_FORK_ENV"
grep -q "^HERDR_BIN_PATH=.*herdr-fork$" "$home/fork-pane.env" || fail "herdr-fork pane HERDR_BIN_PATH is not herdr-fork"
pass "herdr-fork panes point agent hooks at herdr-fork"

if update_output=$("$fork" update 2>&1); then
  fail "herdr-fork update ran"
fi
grep -q "does not self-update" <<<"$update_output" || fail "unexpected herdr-fork update output: $update_output"
pass "herdr-fork refuses to self-update"

if "$fork" integration install claude >/dev/null 2>&1; then
  fail "herdr-fork installed an agent integration"
fi
[ ! -e "$home/.claude" ] || fail "herdr-fork wrote to ~/.claude"
pass "herdr-fork leaves shared agent integrations alone"

"$fork" completion zsh | head -1 | grep -q 'herdr-fork' || fail "herdr-fork completions are not named herdr-fork"
pass "herdr-fork completions are named herdr-fork"

"$fork" server stop >/dev/null
wait "$fork_pid" 2>/dev/null || true
fork_pid=
"$herdr" status server | grep -q '^status: running' || fail "stopping herdr-fork stopped herdr"
grep -qx stock-only <<<"$(workspace_labels "$herdr")" || fail "herdr lost its workspace after herdr-fork stopped"
pass "stopping herdr-fork leaves herdr running"

echo
echo "files under the test HOME:"
find "$home" -mindepth 1 -maxdepth 3 \( -name '*.log' -o -name 'fork-pane.env' \) -prune -o -print | sed "s|^$home|~|" | sort
echo
echo "herdr-fork isolation smoke passed"
