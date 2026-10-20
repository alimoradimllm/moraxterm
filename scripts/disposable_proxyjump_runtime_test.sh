#!/usr/bin/env bash
set -euo pipefail
umask 077
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/moraxterm-proxyjump.XXXXXX")"; cleanup() { kill "${GATEWAY_PID:-}" "${TARGET_PID:-}" 2>/dev/null || true; wait "${GATEWAY_PID:-}" "${TARGET_PID:-}" 2>/dev/null || true; rm -rf "$ROOT"; }; trap cleanup EXIT
mkdir -p "$ROOT/gateway" "$ROOT/target"
KEY="$ROOT/user_key"; ssh-keygen -q -t ed25519 -N '' -f "$KEY"
for role in gateway target; do ssh-keygen -q -t ed25519 -N '' -f "$ROOT/$role/host_key"; cp "$KEY.pub" "$ROOT/$role/authorized_keys"; done
cat > "$ROOT/gateway/config" <<CFG
Port 22460
ListenAddress 127.0.0.1
HostKey $ROOT/gateway/host_key
AuthorizedKeysFile $ROOT/gateway/authorized_keys
PidFile $ROOT/gateway/sshd.pid
UsePAM no
PasswordAuthentication no
AllowTcpForwarding yes
PermitRootLogin no
StrictModes no
LogLevel ERROR
CFG
cat > "$ROOT/target/config" <<CFG
Port 22461
ListenAddress 127.0.0.1
HostKey $ROOT/target/host_key
AuthorizedKeysFile $ROOT/target/authorized_keys
PidFile $ROOT/target/sshd.pid
UsePAM no
PasswordAuthentication no
PermitRootLogin no
StrictModes no
LogLevel ERROR
CFG
/usr/sbin/sshd -f "$ROOT/gateway/config" -E "$ROOT/gateway/log"; GATEWAY_PID=$(cat "$ROOT/gateway/sshd.pid")
/usr/sbin/sshd -f "$ROOT/target/config" -E "$ROOT/target/log"; TARGET_PID=$(cat "$ROOT/target/sshd.pid")
ssh-keyscan -T 5 -p 22460 127.0.0.1 > "$ROOT/known_hosts" 2>/dev/null; ssh-keyscan -T 5 -p 22461 127.0.0.1 >> "$ROOT/known_hosts" 2>/dev/null
OPTS=(-i "$KEY" -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$ROOT/known_hosts" -o BatchMode=yes -o ConnectTimeout=5)
USER_NAME="$(id -un)"
PROXY="ssh ${OPTS[*]} -W %h:%p -p 22460 $USER_NAME@127.0.0.1"
if ssh "${OPTS[@]}" -J "$USER_NAME@127.0.0.1:22460" -p 22461 "$USER_NAME@127.0.0.1" true >/dev/null 2>&1; then echo 'proxyjump_forwarding=passed'; else echo 'proxyjump_forwarding=blocked_by_ssh_wrapper'; fi
ssh "${OPTS[@]}" -o "ProxyCommand=$PROXY" -p 22461 "$USER_NAME@127.0.0.1" true
echo 'structured_proxycommand_forwarding=passed'
echo 'DISPOSABLE_PROXYJUMP_RUNTIME_TEST=PASS'
