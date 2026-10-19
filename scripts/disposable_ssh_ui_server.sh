#!/usr/bin/env bash
# Throwaway loopback sshd for the Moraxterm UI SSH tests (run in Debian WSL; the Windows ssh.exe inside the app connects to it).
# Nothing touches the real ~/.ssh: the client key, known_hosts and ssh config are generated in a temporary Windows-visible folder,
# and the app is pointed at that config with MORAXTERM_TEST_SSH_CONFIG (and, for host-key tests, MORAXTERM_TEST_KNOWN_HOSTS).
# Stays up until the stop file appears or 20 minutes pass.
#
#   wsl -d Debian -- bash scripts/disposable_ssh_ui_server.sh              known_hosts already trusts the server (MultiExec tests)
#   MORAXTERM_TEST_KNOWN_HOSTS_EMPTY=1 wsl -d Debian -- bash ...           known_hosts starts empty (host-key tests)
#
# While it runs, creating the file "rotate" in the temp folder replaces the server's host key and restarts sshd on the same port,
# which is how a real "host key changed" situation is produced. The ready file is updated with {"rotations": N}.
set -euo pipefail
umask 077
PORT="${MORAXTERM_TEST_SSH_UI_PORT:-22226}"
EMPTY="${MORAXTERM_TEST_KNOWN_HOSTS_EMPTY:-0}"
WIN_TEMP="$(wslpath "$(cmd.exe /c 'echo %TEMP%' 2>/dev/null | tr -d '\r')")"
# The default port keeps the original ready-file name; another port gets its own file, so two servers (two sessions) can run at once and
# one can never delete the other's file. A second server on a port that is already in use, or with a ready file already present, refuses
# to start BEFORE it installs its cleanup, so it cannot remove the first server's file or folders.
if [ "$PORT" = "22226" ]; then READY="$WIN_TEMP/moraxterm-ssh-ui-ready.json"; else READY="$WIN_TEMP/moraxterm-ssh-ui-ready-$PORT.json"; fi
if (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; then
  echo "REFUSED: port $PORT is already in use (another test server?). Choose MORAXTERM_TEST_SSH_UI_PORT=<free port>." >&2; exit 3
fi
if [ -e "$READY" ]; then
  echo "REFUSED: $READY already exists (a server is running or died without cleaning up). Remove it only if you are sure no server uses it." >&2; exit 3
fi
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/moraxterm-ssh-ui.XXXXXX")"
SSHD_PID=""
WIN_DIR="$(mktemp -d "$WIN_TEMP/moraxterm-ssh-ui.XXXXXX")"
OWN_READY=0
cleanup() {
  [ -n "$SSHD_PID" ] && kill "$SSHD_PID" 2>/dev/null || true
  [ "$OWN_READY" = "1" ] && rm -f "$READY"   # only a file this server wrote
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
icacls.exe "$(wslpath -w "$WIN_DIR/user_key")" /inheritance:r /grant:r "$(cmd.exe /c 'echo %USERNAME%' 2>/dev/null | tr -d '\r'):R" >/dev/null
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
Subsystem sftp $(ls /usr/lib/openssh/sftp-server /usr/lib/ssh/sftp-server 2>/dev/null | head -1)
CFG
mkdir -p "$ROOT/remote"   # files the SFTP tests send land here (inside the temporary folder, removed with it)
start_sshd() {
  /usr/sbin/sshd -f "$ROOT/srv/sshd_config" -E "$ROOT/srv/log"
  SSHD_PID="$(cat "$ROOT/srv/sshd.pid")"
}
start_sshd
if [ "$EMPTY" = "1" ]; then
  : > "$WIN_DIR/known_hosts"
else
  ssh-keyscan -T 5 -p "$PORT" 127.0.0.1 2>/dev/null > "$WIN_DIR/known_hosts"
  test -s "$WIN_DIR/known_hosts"
fi
cat > "$WIN_DIR/ssh_config" <<CFG
Host *
  IdentityFile $NATIVE/user_key
  IdentitiesOnly yes
  UserKnownHostsFile $NATIVE/known_hosts
  StrictHostKeyChecking yes
CFG
write_ready() {
  printf '{"config": "%s/ssh_config", "known_hosts": "%s/known_hosts", "port": %s, "user": "%s", "dir": "%s", "remote_dir": "%s", "rotations": %s}\n' \
    "$NATIVE" "$NATIVE" "$PORT" "$(id -un)" "$NATIVE" "$ROOT/remote" "$1" > "$READY"
}
ROTATIONS=0
OWN_READY=1
write_ready 0
echo "READY $READY"
for _ in $(seq 1 1200); do
  [ -e "$WIN_DIR/stop" ] && break
  if [ -e "$WIN_DIR/rotate" ]; then
    rm -f "$WIN_DIR/rotate"
    kill "$SSHD_PID" 2>/dev/null || true
    sleep 1
    rm -f "$ROOT/srv/host_key" "$ROOT/srv/host_key.pub"
    ssh-keygen -q -t ed25519 -N '' -f "$ROOT/srv/host_key"
    start_sshd
    ROTATIONS=$((ROTATIONS + 1))
    write_ready "$ROTATIONS"
  fi
  sleep 1
done
echo "STOPPED"
