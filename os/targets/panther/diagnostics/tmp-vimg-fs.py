import struct
V="/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/vendor.img"
f=open(V,"rb"); d=f.read(4096)
print("bytes[0:8]", d[0:8].hex())
print("erofs magic @1024:", d[1024:1028].hex(), "(expect e2 e1 f5 e0)")
print("ext4 magic @1080:", d[1080:1082].hex(), "(expect 53ef)")
print("sqsh @0:", d[0:4], "(expect hsqs)")
# scan for libsitril string anywhere (coarse) in first 64MB
f.seek(0)
chunk=f.read(64*1024*1024)
i=chunk.find(b"libsitril")
print("libsitril first occurrence in first 64MB:", hex(i) if i>=0 else "none")
