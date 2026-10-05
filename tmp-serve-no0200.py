import http.server
import socketserver

class H(http.server.SimpleHTTPRequestHandler):
    def log_message(self, fmt, *args):
        print(fmt % args, flush=True)

with socketserver.TCPServer(("172.31.7.2", 38128), H) as s:
    print("ready", flush=True)
    s.serve_forever()
