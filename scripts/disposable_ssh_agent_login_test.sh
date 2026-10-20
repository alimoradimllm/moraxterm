#!/usr/bin/env bash
set -euo pipefail
umask 077
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/moraxterm-agent.XXXXXX")"
cleanup() { kill "${SSHD_PID:-}" 2>/dev/null || true; wait "${SSHD_PID:-}" 2>/dev/null || true; [ -n "${SSH_AGENT_PID:-}" ] && ssh-agent -k >/dev/null 2>&1 || true; rm -rf "$ROOT"; }
trap cleanup EXIT
mkdir -p "$ROOT/server"
USER_KEY="$ROOT/user_key"
ssh-keygen -q -t ed25519 -N '' -f "$USER_KEY"
cp "$USER_KEY.pub" "$ROOT/server/authorized_keys"
ssh-keygen -q -t ed25519 -N '' -f "$ROOT/server/host_key"
USER_NAME="$(id -un)"
cat > "$ROOT/server/sshd_config" <<CFG
Port 22440
ListenAddress 127.0.0.1
HostKey $ROOT/server/host_key
AuthorizedKeysFile $ROOT/server/authorized_keys
PidFile $ROOT/server/sshd.pid
UsePAM no
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
StrictModes no
LogLevel ERROR
CFG
/usr/sbin/sshd -f "$ROOT/server/sshd_config" -E "$ROOT/server/log"; SSHD_PID=$(cat "$ROOT/server/sshd.pid")
ssh-keyscan -T 5 -p 22440 127.0.0.1 > "$ROOT/known_hosts" 2>/dev/null
eval "$(ssh-agent -s)" >/dev/null
ssh-add -q "$USER_KEY"
SSH_AUTH_SOCK="$SSH_AUTH_SOCK" ssh -o IdentitiesOnly=no -o IdentityAgent="$SSH_AUTH_SOCK" -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$ROOT/known_hosts" -o BatchMode=yes -o ConnectTimeout=5 -p 22440 "$USER_NAME@127.0.0.1" 'printf AGENT_LOGIN_OK'
echo
echo 'ssh_agent_authenticated_login=passed'
echo 'DISPOSABLE_SSH_AGENT_LOGIN_TEST=PASS'
