"""Loopback HTTP/HTTPS fixture server with byte-range support.

Fallback for the Caddy file_server defined in test-env/Caddyfile when Caddy is
not installed on the recording host. Contract is the same: static files under
test-env/fixtures/http, HTTP/1.1 keep-alive, HEAD, Range -> 206, TLS with the
test-only CA, and /trunc/<file>/<bytes> from RECORD-CASES §7-1.
"""

import os
import re
import ssl
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = os.path.dirname(os.path.abspath(__file__))
FIXTURES = os.path.join(ROOT, "fixtures", "http")
CERTS = os.path.join(ROOT, "certs")
HTTP_PORT = int(os.environ.get("HTTP_PORT", "18080"))
HTTPS_PORT = int(os.environ.get("HTTPS_PORT", "18443"))
RANGE = re.compile(r"^bytes=(\d*)-(\d*)$")


class FixtureHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):
        pass

    def _path(self):
        rel = self.path.split("?", 1)[0].lstrip("/")
        return rel, os.path.normpath(os.path.join(FIXTURES, rel))

    def _send(self, head_only=False):
        rel, path = self._path()

        # Truncating endpoint: full Content-Length, N bytes, then disconnect.
        parts = rel.split("/")
        if len(parts) == 3 and parts[0] == "trunc" and parts[2].isdigit():
            target = os.path.join(FIXTURES, os.path.basename(parts[1]))
            if os.path.basename(parts[1]) != parts[1] or not os.path.isfile(target):
                self.send_error(404)
                return
            size = os.path.getsize(target)
            self.send_response(200)
            self.send_header("Content-Length", str(size))
            self.send_header("Content-Type", "application/octet-stream")
            self.end_headers()
            if not head_only:
                with open(target, "rb") as handle:
                    self.wfile.write(handle.read(min(int(parts[2]), size)))
                self.wfile.flush()
                self.close_connection = True
            return

        if not path.startswith(FIXTURES + os.sep) or not os.path.isfile(path):
            self.send_error(404)
            return

        size = os.path.getsize(path)
        start, end = 0, size - 1
        status = 200
        header = self.headers.get("Range")
        match = RANGE.match(header.strip()) if header else None
        if match:
            first, last = match.groups()
            if first:
                start = int(first)
                end = int(last) if last else size - 1
            elif last:  # suffix range
                start = max(0, size - int(last))
            end = min(end, size - 1)
            if start > end or start >= size:
                self.send_response(416)
                self.send_header("Content-Range", f"bytes */{size}")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            status = 206

        length = max(0, end - start + 1)
        self.send_response(status)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(length))
        self.send_header("Accept-Ranges", "bytes")
        if status == 206:
            self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
        self.end_headers()
        if head_only:
            return
        with open(path, "rb") as handle:
            handle.seek(start)
            remaining = length
            while remaining:
                chunk = handle.read(min(65536, remaining))
                if not chunk:
                    break
                self.wfile.write(chunk)
                remaining -= len(chunk)

    def do_GET(self):
        self._send()

    def do_HEAD(self):
        self._send(head_only=True)


def serve(port, tls=False):
    server = ThreadingHTTPServer(("127.0.0.1", port), FixtureHandler)
    if tls:
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(
            os.path.join(CERTS, "server.crt"), os.path.join(CERTS, "server.key")
        )
        server.socket = context.wrap_socket(server.socket, server_side=True)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    print(f"fixture server on 127.0.0.1:{port} tls={tls}", flush=True)
    return server


if __name__ == "__main__":
    servers = [serve(HTTP_PORT), serve(HTTPS_PORT, tls=True)]
    print("fixture servers ready", flush=True)
    try:
        threading.Event().wait()
    except KeyboardInterrupt:
        for server in servers:
            server.shutdown()
        sys.exit(0)
