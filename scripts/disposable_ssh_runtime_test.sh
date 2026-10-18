#!/usr/bin/env bash
set -euo pipefail
umask 077
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/moraxterm-ssh-test.XXXXXX")"
TARGET_PORT=22222
GATEWAY_PORT=22223
USER_KEY="$ROOT/user_key"
KNOWN="$ROOT/known_hosts"
cleanup() {
  kill "${TARGET_PID:-}" "${GATEWAY_PID:-}" 2>/dev/null || true
  wait "${TARGET_PID:-}" "${GATEWAY_PID:-}" 2>/dev/null || true
  if [ -n "${SSH_AGENT_PID:-}" ]; then ssh-agent -k >/dev/null 2>&1 || true; fi
  rm -rf "$ROOT"
}
trap cleanup EXIT
mkdir -p "$ROOT/target" "$ROOT/gateway"
ssh-keygen -q -t ed25519 -N '' -f "$USER_KEY"
eval "$(ssh-agent -s)" >/dev/null
ssh-add -q "$USER_KEY"
for role in target gateway; do
  ssh-keygen -q -t ed25519 -N '' -f "$ROOT/$role/ssh_host_ed25519_key"
  cp "$USER_KEY.pub" "$ROOT/$role/authorized_keys"
  cat > "$ROOT/$role/sshd_config" <<CFG
Port $([ "$role" = target ] && echo "$TARGET_PORT" || echo "$GATEWAY_PORT")
ListenAddress 127.0.0.1
HostKey $ROOT/$role/ssh_host_ed25519_key
AuthorizedKeysFile $ROOT/$role/authorized_keys
PidFile $ROOT/$role/sshd.pid
UsePAM no
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
AllowTcpForwarding yes
X11Forwarding no
StrictModes no
LogLevel ERROR
CFG
done
/usr/sbin/sshd -f "$ROOT/target/sshd_config" -E "$ROOT/target/log"
/usr/sbin/sshd -f "$ROOT/gateway/sshd_config" -E "$ROOT/gateway/log"
TARGET_PID=$(cat "$ROOT/target/sshd.pid")
GATEWAY_PID=$(cat "$ROOT/gateway/sshd.pid")
SSH=(ssh -i "$USER_KEY" -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$KNOWN" -o BatchMode=yes -o ConnectTimeout=5 -o ExitOnForwardFailure=yes)
GATEWAY_SCAN=$(ssh-keyscan -T 5 -p "$GATEWAY_PORT" 127.0.0.1 2>/dev/null)
TARGET_SCAN=$(ssh-keyscan -T 5 -p "$TARGET_PORT" localhost 2>/dev/null)
test -n "$GATEWAY_SCAN"
test -n "$TARGET_SCAN"
printf '%s\n' "$TARGET_SCAN" > "$ROOT/target_scan"
printf '%s\n' "$GATEWAY_SCAN" > "$ROOT/gateway_scan"
FINGERPRINT=$(ssh-keygen -lf "$ROOT/target_scan" | awk '{print $2}')
printf 'target_fingerprint=%s\n' "$FINGERPRINT"
if "${SSH[@]}" -p "$TARGET_PORT" ali@127.0.0.1 true >/dev/null 2>&1; then
  echo 'ERROR strict connection unexpectedly trusted unknown host'
  exit 1
fi
echo 'strict_unknown_host=rejected'
cat "$ROOT/gateway_scan" "$ROOT/target_scan" > "$KNOWN"
mkdir -p "$ROOT/home/.ssh"
cp "$KNOWN" "$ROOT/home/.ssh/known_hosts"
cat > "$ROOT/home/.ssh/config" <<CFG
Host *
  IdentityFile $USER_KEY
  IdentitiesOnly yes
  UserKnownHostsFile $KNOWN
  StrictHostKeyChecking yes
  BatchMode yes
  ConnectTimeout 5
CFG
chmod 600 "$ROOT/home/.ssh/config" "$ROOT/home/.ssh/known_hosts"
export HOME="$ROOT/home"
"${SSH[@]}" -p "$TARGET_PORT" ali@localhost true
 echo 'approved_key_connection=passed'
PROXY_COMMAND="ssh -F $ROOT/home/.ssh/config -W %h:%p -p $GATEWAY_PORT ali@127.0.0.1"
"${SSH[@]}" -o "ProxyCommand=$PROXY_COMMAND" -p "$TARGET_PORT" ali@localhost true
echo 'gateway_forwarding=passed'
kill "$TARGET_PID"
wait "$TARGET_PID" 2>/dev/null || true
rm -f "$ROOT/target/ssh_host_ed25519_key" "$ROOT/target/ssh_host_ed25519_key.pub"
ssh-keygen -q -t ed25519 -N '' -f "$ROOT/target/ssh_host_ed25519_key"
/usr/sbin/sshd -f "$ROOT/target/sshd_config" -E "$ROOT/target/log"
TARGET_PID=$(cat "$ROOT/target/sshd.pid")
if "${SSH[@]}" -p "$TARGET_PORT" ali@127.0.0.1 true >/dev/null 2>&1; then
  echo 'ERROR changed host key unexpectedly trusted'
  exit 1
fi
echo 'changed_host_key=rejected'
if [ -n "${MORAXTERM_SSH_HOLD_SECONDS:-}" ]; then
  sleep "$MORAXTERM_SSH_HOLD_SECONDS"
fi
echo 'DISPOSABLE_SSH_RUNTIME_TEST=PASS'
