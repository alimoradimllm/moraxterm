#!/usr/bin/env bash
set -euo pipefail
umask 077
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/moraxterm-hostkey.XXXXXX")"
cleanup() { kill "${SSHD_PID:-}" 2>/dev/null || true; wait "${SSHD_PID:-}" 2>/dev/null || true; rm -rf "$ROOT"; }
trap cleanup EXIT
mkdir -p "$ROOT/server"
USER_KEY="$ROOT/user_key"
ssh-keygen -q -t ed25519 -N '' -f "$USER_KEY"
cp "$USER_KEY.pub" "$ROOT/server/authorized_keys"
for spec in rsa:2048 ecdsa:256; do
  type="${spec%%:*}"; bits="${spec##*:}"; port=$((22400 + ${#type}))
  key="$ROOT/server/ssh_host_${type}_key"
  ssh-keygen -q -t "$type" -b "$bits" -N '' -f "$key"
  cat > "$ROOT/server/sshd_config" <<CFG
Port $port
ListenAddress 127.0.0.1
HostKey $key
AuthorizedKeysFile $ROOT/server/authorized_keys
PidFile $ROOT/server/sshd.pid
UsePAM no
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
StrictModes no
LogLevel ERROR
CFG
  /usr/sbin/sshd -f "$ROOT/server/sshd_config" -E "$ROOT/server/log"
  SSHD_PID=$(cat "$ROOT/server/sshd.pid")
  known="$ROOT/known_${type}"
  ssh-keyscan -T 5 -p "$port" 127.0.0.1 > "$known" 2>/dev/null
  test -s "$known"
  ssh -i "$USER_KEY" -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$known" -o BatchMode=yes -o ConnectTimeout=5 -p "$port" ali@127.0.0.1 true
  echo "${type}_hostkey_connection=passed"
  kill "$SSHD_PID"; wait "$SSHD_PID" 2>/dev/null || true; unset SSHD_PID
done
echo 'DISPOSABLE_SSH_HOSTKEY_ALGORITHMS_TEST=PASS'
