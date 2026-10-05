import http.server
import socketserver
import os

os.chdir(r"C:\Users\Admin\Projects\saaios-som\os\targets\panther\diagnostics")
print("serving", os.getcwd(), "on 0.0.0.0:8765", flush=True)
socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("0.0.0.0", 8765), http.server.SimpleHTTPRequestHandler) as httpd:
    httpd.serve_forever()
