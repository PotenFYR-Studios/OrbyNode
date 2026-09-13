#!/usr/bin/env sh
# OrbyNode uninstaller. Removes the installed binary and, optionally, the
# data directory (~/.orbynode). Data removal is opt-in; configuration and
# durable state are preserved by default so a later reinstall can reuse them.
#
# Usage:
#   scripts/uninstall.sh [--purge] [--force]
#
#   --purge  also delete ~/.orbynode (configuration, database, worktrees)
#   --force  never prompt; defaults apply (keep data unless --purge)
#
# The daemon is stopped gracefully first if it is running under the user.
set -eu

INSTALL_DIR="${ORBYNODE_INSTALL_DIR:-$HOME/.local/bin}"
DATA_DIR="${ORBYNODE_DATA_DIR:-$HOME/.orbynode}"
PURGE=0
FORCE=0

for arg in "$@"; do
  case "$arg" in
    --purge) PURGE=1 ;;
    --force) FORCE=1 ;;
    -h|--help)
      printf 'usage: uninstall.sh [--purge] [--force]\n'
      printf '  --purge  also remove %s (config, database, worktrees)\n' "$DATA_DIR"
      printf '  --force  do not prompt; keep data unless --purge given\n'
      exit 0
      ;;
    *)
      printf 'unknown option: %s\n' "$arg" >&2
      exit 2
      ;;
  esac
done

log() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }

# 1. Stop a running daemon for this user, gracefully, then confirm exit.
find_pid() {
  # Exact process-name match; verify the executable really lives in the
  # install directory via /proc where available, so unrelated processes with
  # the same name (or command lines mentioning the path) are never killed.
  for pid in $(pgrep -x orbynode 2>/dev/null; pgrep -x orbynode-daemon 2>/dev/null); do
    exe="$(readlink "/proc/$pid/exe" 2>/dev/null || true)"
    case "$exe" in
      "$INSTALL_DIR"/*) printf '%s\n' "$pid"; return 0 ;;
      "") printf '%s\n' "$pid"; return 0 ;;  # no /proc (macOS/BSD): name match only
    esac
  done
  return 0
}

PID="$(find_pid)"
if [ -n "$PID" ]; then
  log "stopping daemon (pid $PID)"
  kill "$PID" 2>/dev/null || true
  i=0
  while [ "$i" -lt 20 ]; do
    kill -0 "$PID" 2>/dev/null || break
    i=$((i + 1))
    sleep 0.5
  done
  if kill -0 "$PID" 2>/dev/null; then
    warn "daemon did not exit after 10s; sending SIGKILL"
    kill -9 "$PID" 2>/dev/null || true
    sleep 1
  fi
else
  log "no running daemon found"
fi

# 2. Remove installed binary.
REMOVED=0
for name in orbynode orbynode-daemon; do
  if [ -f "$INSTALL_DIR/$name" ]; then
    rm -f "$INSTALL_DIR/$name"
    log "removed $INSTALL_DIR/$name"
    REMOVED=1
  fi
done
[ "$REMOVED" = "1" ] || warn "no binary found in $INSTALL_DIR"

# 3. Data directory: keep by default, purge only when asked.
if [ -d "$DATA_DIR" ]; then
  keep=1
  if [ "$PURGE" = "1" ]; then
    keep=0
  elif [ "$FORCE" != "1" ]; then
    # Interactive decision: config reuse is the default answer.
    printf 'Keep configuration and data in %s for reuse after reinstall? [Y/n] ' "$DATA_DIR"
    read -r answer </dev/tty 2>/dev/null || answer=y
    case "$answer" in
      n|N) keep=0 ;;
      *) keep=1 ;;
    esac
  fi

  if [ "$keep" = "0" ]; then
    # WAL/SHM sidecars must go with the database or a reinstall will see a
    # torn WAL state.
    rm -rf "$DATA_DIR"
    log "removed $DATA_DIR (config, database, worktrees)"
  else
    log "kept $DATA_DIR (config preserved; reinstall will reuse it)"
  fi
else
  log "no data directory at $DATA_DIR"
fi

log "uninstall complete"
