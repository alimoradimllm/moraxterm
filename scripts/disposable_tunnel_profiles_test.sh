#!/usr/bin/env bash
set -euo pipefail
umask 077
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/moraxterm-tunnels.XXXXXX")"
cleanup() { kill "${SSH_PID:-}" "${SSHD_PID:-}" "${ECHO_PID:-}" 2>/dev/null || true; wait "${SSH_PID:-}" "${SSHD_PID:-}" "${ECHO_PID:-}" 2>/dev/null || true; rm -rf "$ROOT"; }
trap cleanup EXIT
mkdir -p "$ROOT/server"
USER_KEY="$ROOT/user_key"; ssh-keygen -q -t ed25519 -N '' -f "$USER_KEY"; cp "$USER_KEY.pub" "$ROOT/server/authorized_keys"
python3 - "$ROOT" <<'PY' &
import socket, sys
root=sys.argv[1]; s=socket.socket(); s.bind(('127.0.0.1',0)); s.listen(); open(root+'/echo.port','w').write(str(s.getsockname()[1]));
while True:
    c,_=s.accept(); c.sendall(b'MORAXTERM_TUNNEL_OK\n'); c.close()
PY
ECHO_PID=$!
while [ ! -s "$ROOT/echo.port" ]; do sleep .05; done
ECHO_PORT=$(cat "$ROOT/echo.port")
ssh-keygen -q -t ed25519 -N '' -f "$ROOT/server/ssh_host_ed25519_key"
cat > "$ROOT/server/sshd_config" <<CFG
Port 22420
ListenAddress 127.0.0.1
HostKey $ROOT/server/ssh_host_ed25519_key
AuthorizedKeysFile $ROOT/server/authorized_keys
PidFile $ROOT/server/sshd.pid
UsePAM no
PasswordAuthentication no
PermitRootLogin no
AllowTcpForwarding yes
StrictModes no
LogLevel ERROR
CFG
/usr/sbin/sshd -f "$ROOT/server/sshd_config" -E "$ROOT/server/log"; SSHD_PID=$(cat "$ROOT/server/sshd.pid")
ssh-keyscan -T 5 -p 22420 127.0.0.1 > "$ROOT/known_hosts" 2>/dev/null
SSH_OPTS=(-i "$USER_KEY" -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$ROOT/known_hosts" -o BatchMode=yes -o ConnectTimeout=5)
ssh "${SSH_OPTS[@]}" -N -L 22421:127.0.0.1:"$ECHO_PORT" -p 22420 ali@127.0.0.1 & SSH_PID=$!
ssh "${SSH_OPTS[@]}" -N -L 22422:127.0.0.1:"$ECHO_PORT" -p 22420 ali@127.0.0.1 & SSH_PID_2=$!
sleep 1
for port in 22421 22422; do
  timeout 5 bash -c "</dev/tcp/127.0.0.1/$port" >/dev/null
  echo "tunnel_profile_$port=passed"
done
kill "$SSH_PID" "$SSH_PID_2" 2>/dev/null || true
echo 'DISPOSABLE_TUNNEL_PROFILES_TEST=PASS'
