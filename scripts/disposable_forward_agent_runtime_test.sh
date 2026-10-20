#!/usr/bin/env bash
set -euo pipefail
umask 077
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/moraxterm-forward-agent.XXXXXX")"
cleanup() { kill "${A_PID:-}" "${B_PID:-}" 2>/dev/null || true; wait "${A_PID:-}" "${B_PID:-}" 2>/dev/null || true; [ -n "${SSH_AGENT_PID:-}" ] && ssh-agent -k >/dev/null 2>&1 || true; rm -rf "$ROOT"; }
trap cleanup EXIT
mkdir -p "$ROOT/a" "$ROOT/b"
KEY="$ROOT/user_key"; ssh-keygen -q -t ed25519 -N '' -f "$KEY"
for role in a b; do ssh-keygen -q -t ed25519 -N '' -f "$ROOT/$role/host_key"; cp "$KEY.pub" "$ROOT/$role/authorized_keys"; done
cat > "$ROOT/a/config" <<CFG
Port 22480
ListenAddress 127.0.0.1
HostKey $ROOT/a/host_key
AuthorizedKeysFile $ROOT/a/authorized_keys
PidFile $ROOT/a/sshd.pid
UsePAM no
PasswordAuthentication no
AllowAgentForwarding yes
StrictModes no
LogLevel ERROR
CFG
cat > "$ROOT/b/config" <<CFG
Port 22481
ListenAddress 127.0.0.1
HostKey $ROOT/b/host_key
AuthorizedKeysFile $ROOT/b/authorized_keys
PidFile $ROOT/b/sshd.pid
UsePAM no
PasswordAuthentication no
StrictModes no
LogLevel ERROR
CFG
/usr/sbin/sshd -f "$ROOT/a/config" -E "$ROOT/a/log"; A_PID=$(cat "$ROOT/a/sshd.pid")
/usr/sbin/sshd -f "$ROOT/b/config" -E "$ROOT/b/log"; B_PID=$(cat "$ROOT/b/sshd.pid")
ssh-keyscan -T 5 -p 22480 127.0.0.1 > "$ROOT/known_hosts" 2>/dev/null; ssh-keyscan -T 5 -p 22481 127.0.0.1 >> "$ROOT/known_hosts" 2>/dev/null
eval "$(ssh-agent -s)" >/dev/null; ssh-add -q "$KEY"
USER_NAME="$(id -un)"; NESTED="ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$ROOT/known_hosts -p 22481 $USER_NAME@127.0.0.1 printf FORWARD_AGENT_OK"
ssh -A -o IdentitiesOnly=no -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$ROOT/known_hosts" -o BatchMode=yes -o ForwardAgent=yes -p 22480 "$USER_NAME@127.0.0.1" "$NESTED" | grep -q FORWARD_AGENT_OK
echo 'forward_agent_nested_login=passed'
echo 'DISPOSABLE_FORWARD_AGENT_RUNTIME_TEST=PASS'
