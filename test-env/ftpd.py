"""Read-only loopback FTP fixture server.

Uses pyftpdlib's ThreadedFTPServer directly: the bundled `python3 -m pyftpdlib
--concurrency multi-thread` CLI passes `worker_processes` to a server class that
does not accept it (pyftpdlib 2.2.0), and the default async server occasionally
stalls a control connection when `app replay` runs many cases in parallel.
Credentials come from test-env/ports.env (approved synthetic constants).
"""

import os

from pyftpdlib.authorizers import DummyAuthorizer
from pyftpdlib.handlers import FTPHandler
from pyftpdlib.servers import ThreadedFTPServer

ROOT = os.path.dirname(os.path.abspath(__file__))
PORT = int(os.environ.get("FTP_PORT", "1821"))
USER = os.environ.get("FTP_USER", "aria2test")
PASS = os.environ.get("FTP_PASS", "test-only")
DIRECTORY = os.path.join(ROOT, "fixtures", "ftp")

# 'r' + 'l' (read and list) are needed for RETR, SIZE and directory listing.
# No write/delete permission is granted: the fixture tree stays read-only.
authorizer = DummyAuthorizer()
authorizer.add_user(USER, PASS, DIRECTORY, perm="rl")
handler = FTPHandler
handler.authorizer = authorizer
handler.banner = "aria2-rs loopback test FTP server"

server = ThreadedFTPServer(("127.0.0.1", PORT), handler)
print(f"FTP server on 127.0.0.1:{PORT} (read-only)", flush=True)
try:
    server.serve_forever()
except KeyboardInterrupt:
    server.close_all()
