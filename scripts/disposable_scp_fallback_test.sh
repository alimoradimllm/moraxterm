#!/usr/bin/env bash
set -euo pipefail
umask 077
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/moraxterm-scp.XXXXXX")"
USER_NAME="$(id -un)"
cleanup() { kill "${SSHD_PID:-}" 2>/dev/null || true; wait "${SSHD_PID:-}" 2>/dev/null || true; rm -rf "$ROOT"; }
trap cleanup EXIT
mkdir -p "$ROOT/server" "$ROOT/local" "$ROOT/remote"
SFTP_SERVER="$(ls /usr/lib/openssh/sftp-server /usr/lib/ssh/sftp-server /usr/libexec/openssh/sftp-server 2>/dev/null | head -1)"
ssh-keygen -q -t ed25519 -N '' -f "$ROOT/user_key"; ssh-keygen -q -t ed25519 -N '' -f "$ROOT/server/host_key"
cp "$ROOT/user_key.pub" "$ROOT/server/authorized_keys"
cat > "$ROOT/server/sshd_config" <<CFG
Port 22430
ListenAddress 127.0.0.1
HostKey $ROOT/server/host_key
AuthorizedKeysFile $ROOT/server/authorized_keys
PidFile $ROOT/server/sshd.pid
UsePAM no
PasswordAuthentication no
StrictModes no
LogLevel ERROR
Subsystem sftp $SFTP_SERVER
CFG
/usr/sbin/sshd -f "$ROOT/server/sshd_config" -E "$ROOT/server/log"; SSHD_PID=$(cat "$ROOT/server/sshd.pid")
ssh-keyscan -T 5 -p 22430 127.0.0.1 > "$ROOT/known_hosts" 2>/dev/null
OPTS=(-i "$ROOT/user_key" -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$ROOT/known_hosts" -o BatchMode=yes -o ConnectTimeout=5)
head -c 131072 /dev/urandom > "$ROOT/local/upload.bin"
scp -q "${OPTS[@]}" -P 22430 "$ROOT/local/upload.bin" "$USER_NAME@127.0.0.1:$ROOT/remote/upload.bin"
cmp "$ROOT/local/upload.bin" "$ROOT/remote/upload.bin"
scp -q "${OPTS[@]}" -P 22430 "$USER_NAME@127.0.0.1:$ROOT/remote/upload.bin" "$ROOT/local/download.bin"
cmp "$ROOT/local/upload.bin" "$ROOT/local/download.bin"
echo 'scp_fallback_upload=passed'
echo 'scp_fallback_download=passed'
echo 'DISPOSABLE_SCP_FALLBACK_TEST=PASS'
