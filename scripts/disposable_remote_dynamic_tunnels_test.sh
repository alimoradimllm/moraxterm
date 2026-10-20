#!/usr/bin/env bash
set -euo pipefail
umask 077
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/moraxterm-tunnels-rd.XXXXXX")"
cleanup() { kill "${SSH_PID:-}" "${SSHD_PID:-}" "${HTTP_PID:-}" 2>/dev/null || true; wait "${SSH_PID:-}" "${SSHD_PID:-}" "${HTTP_PID:-}" 2>/dev/null || true; rm -rf "$ROOT"; }
trap cleanup EXIT
mkdir -p "$ROOT/server"
USER_KEY="$ROOT/user_key"; ssh-keygen -q -t ed25519 -N '' -f "$USER_KEY"; cp "$USER_KEY.pub" "$ROOT/server/authorized_keys"; ssh-keygen -q -t ed25519 -N '' -f "$ROOT/server/host_key"
python3 - "$ROOT" <<'PY' &
import http.server, socketserver, sys
root=sys.argv[1]
class H(http.server.BaseHTTPRequestHandler):
  def do_GET(self): self.send_response(200); self.end_headers(); self.wfile.write(b'MORAXTERM_TUNNEL_MODE_OK\n')
  def log_message(self,*a): pass
s=socketserver.TCPServer(('127.0.0.1',0),H); open(root+'/http.port','w').write(str(s.server_address[1])); s.serve_forever()
PY
HTTP_PID=$!; while [ ! -s "$ROOT/http.port" ]; do sleep .05; done; HTTP_PORT=$(cat "$ROOT/http.port")
cat > "$ROOT/server/sshd_config" <<CFG
Port 22450
ListenAddress 127.0.0.1
HostKey $ROOT/server/host_key
AuthorizedKeysFile $ROOT/server/authorized_keys
PidFile $ROOT/server/sshd.pid
UsePAM no
PasswordAuthentication no
PermitRootLogin no
AllowTcpForwarding yes
GatewayPorts no
StrictModes no
LogLevel ERROR
Subsystem sftp /usr/lib/openssh/sftp-server
CFG
/usr/sbin/sshd -f "$ROOT/server/sshd_config" -E "$ROOT/server/log"; [ -f "$ROOT/server/sshd.pid" ] || { cat "$ROOT/server/log"; exit 1; }; SSHD_PID=$(cat "$ROOT/server/sshd.pid")
ssh-keyscan -T 5 -p 22450 127.0.0.1 > "$ROOT/known_hosts" 2>/dev/null
OPTS=(-i "$USER_KEY" -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$ROOT/known_hosts" -o BatchMode=yes -o ConnectTimeout=5)
ssh "${OPTS[@]}" -N -R 22451:127.0.0.1:"$HTTP_PORT" -p 22450 "$(id -un)@127.0.0.1" & SSH_PID=$!; sleep 1
ssh "${OPTS[@]}" -p 22450 "$(id -un)@127.0.0.1" "curl -fsS http://127.0.0.1:22451" | grep -q MORAXTERM_TUNNEL_MODE_OK
echo 'remote_forwarding=passed'; kill "$SSH_PID" 2>/dev/null || true
ssh "${OPTS[@]}" -N -D 22452 -p 22450 "$(id -un)@127.0.0.1" & SSH_PID=$!; sleep 1
curl --silent --show-error --fail --socks5-hostname 127.0.0.1:22452 "http://127.0.0.1:$HTTP_PORT" | grep -q MORAXTERM_TUNNEL_MODE_OK
echo 'dynamic_forwarding=passed'; echo 'DISPOSABLE_REMOTE_DYNAMIC_TUNNELS_TEST=PASS'
