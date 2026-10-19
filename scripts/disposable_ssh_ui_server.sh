#!/usr/bin/env bash
# Throwaway loopback sshd for the Moraxterm UI SSH tests (run in Debian WSL; the Windows ssh.exe inside the app connects to it).
# Nothing touches the real ~/.ssh: the client key, known_hosts and ssh config are generated in a temporary Windows-visible folder,
# and the app is pointed at that config with MORAXTERM_TEST_SSH_CONFIG. Stays up until the stop file appears or 20 minutes pass.
#
#   wsl -d Debian -- bash scripts/disposable_ssh_ui_server.sh          (leave it running)
#   python scripts/ui_ssh_multiexec_test.py                           (from Windows, reads the ready file)
set -euo pipefail
umask 077
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/moraxterm-ssh-ui.XXXXXX")"
PORT="${MORAXTERM_TEST_SSH_UI_PORT:-22226}"
SSHD_PID=""
WIN_TEMP="$(wslpath "$(cmd.exe /c 'echo %TEMP%' 2>/dev/null | tr -d '\r')")"
WIN_DIR="$(mktemp -d "$WIN_TEMP/moraxterm-ssh-ui.XXXXXX")"
READY="$WIN_TEMP/moraxterm-ssh-ui-ready.json"
cleanup() {
  [ -n "$SSHD_PID" ] && kill "$SSHD_PID" 2>/dev/null || true
  rm -f "$READY"
  rm -rf "$WIN_DIR" "$ROOT"
}
trap cleanup EXIT

NATIVE="$(wslpath -m "$WIN_DIR")"
mkdir -p "$ROOT/srv"
ssh-keygen -q -t ed25519 -N '' -f "$WIN_DIR/user_key"
ssh-keygen -q -t ed25519 -N '' -f "$ROOT/srv/host_key"
cp "$WIN_DIR/user_key.pub" "$ROOT/srv/authorized_keys"
# Windows ssh.exe refuses a private key whose ACL lets other accounts read it ("UNPROTECTED PRIVATE KEY FILE"). Keep only the current
# user, read-only. This is the throwaway test key in the temporary folder, nothing else.
icacls.exe "$(wslpath -w "$WIN_DIR/user_key")" /inheritance:r /grant:r "$(cmd.exe /c 'echo %USERNAME%' 2>/dev/null | tr -d ''):R" >/dev/null
cat > "$ROOT/srv/sshd_config" <<CFG
Port $PORT
ListenAddress 127.0.0.1
HostKey $ROOT/srv/host_key
AuthorizedKeysFile $ROOT/srv/authorized_keys
PidFile $ROOT/srv/sshd.pid
UsePAM no
PasswordAuthentication no
KbdInteractiveAuthentication no
StrictModes no
PrintMotd no
LogLevel ERROR
CFG
/usr/sbin/sshd -f "$ROOT/srv/sshd_config" -E "$ROOT/srv/log"
SSHD_PID="$(cat "$ROOT/srv/sshd.pid")"
ssh-keyscan -T 5 -p "$PORT" 127.0.0.1 2>/dev/null > "$WIN_DIR/known_hosts"
test -s "$WIN_DIR/known_hosts"
cat > "$WIN_DIR/ssh_config" <<CFG
Host *
  IdentityFile $NATIVE/user_key
  IdentitiesOnly yes
  UserKnownHostsFile $NATIVE/known_hosts
  StrictHostKeyChecking yes
CFG
printf '{"config": "%s/ssh_config", "port": %s, "user": "%s", "dir": "%s"}\n' "$NATIVE" "$PORT" "$(id -un)" "$NATIVE" > "$READY"
echo "READY $READY"
for _ in $(seq 1 240); do
  [ -e "$WIN_DIR/stop" ] && break
  sleep 5
done
echo "STOPPED"
