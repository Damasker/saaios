import socket, sys
out = sys.argv[1]
srv = socket.socket(); srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(("172.31.7.2", 9876)); srv.listen(1)
print("LISTEN 172.31.7.2:9876", flush=True)
conn, addr = srv.accept()
print("PEER", addr, flush=True)
n = 0
with open(out, "wb") as f:
    while True:
        b = conn.recv(1024 * 1024)
        if not b: break
        f.write(b); n += len(b)
        if n % (8 * 1024 * 1024) == 0: print("got", n, flush=True)
conn.close(); srv.close()
print("DONE", n, flush=True)
