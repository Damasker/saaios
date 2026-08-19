# SaaiOS native boot (Galaxy A12 SM-A127F first target)
# Host-only until you explicitly flash from Download mode.

STOCK ?= os/images/stock/A127FXXSDDXJ2
BUILD := os/build
STOCK_UNPACK := $(BUILD)/stock-boot
RAMDISK := $(BUILD)/ramdisk
OUT := $(BUILD)/boot-v011.img
VBMETA_OUT := $(BUILD)/vbmeta-patched.img
BUSYBOX ?= os/third_party/busybox-aarch64
BUSYBOX_DEB_URL ?= https://deb.debian.org/debian/pool/main/b/busybox/busybox-static_1.37.0-6+b9_arm64.deb
CROSS ?= aarch64-linux-gnu-
# BOOT partition on SM-A127F is 44 MiB; stock boot.img is padded to this with AVB.
BOOT_PART_SIZE := 46137344
BL_ZIP ?= /srv/media/BL_A127FXXSDDXJ2_QB86601342_REV00_user_low_ship_MULTI_CERT.tar.md5.zip
MEDIA ?= /srv/media

.PHONY: all unpack init ramdisk boot.img vbmeta odin-tar restore flash check-stock busybox \
	up_param up_param-flash

all: odin-tar

check-stock:
	@test -f $(STOCK)/boot.img || { echo "missing $(STOCK)/boot.img"; exit 1; }
	@test -f $(STOCK)/vbmeta.img || { echo "missing $(STOCK)/vbmeta.img"; exit 1; }

unpack: check-stock
	python3 os/bootimg/bootimg.py unpack $(STOCK)/boot.img -o $(STOCK_UNPACK)

$(BUILD)/init: os/init/init.c
	mkdir -p $(BUILD)
	$(CROSS)gcc -ffreestanding -nostdlib -static -Os \
		-fno-stack-protector -fno-asynchronous-unwind-tables \
		-Wl,-e,_start -Wl,--build-id=none -s -o $@ os/init/init.c

$(BUSYBOX):
	mkdir -p os/third_party $(BUILD)/busybox-deb
	wget -q -O $(BUILD)/busybox-static.deb "$(BUSYBOX_DEB_URL)"
	dpkg-deb -x $(BUILD)/busybox-static.deb $(BUILD)/busybox-deb
	cp $(BUILD)/busybox-deb/usr/bin/busybox $(BUSYBOX)
	chmod 0755 $(BUSYBOX)

busybox: $(BUSYBOX)

init: $(BUILD)/init

ramdisk: $(BUILD)/init $(BUSYBOX)
	rm -rf $(RAMDISK)
	mkdir -p $(RAMDISK)/dev $(RAMDISK)/proc $(RAMDISK)/sys $(RAMDISK)/tmp $(RAMDISK)/run \
		$(RAMDISK)/bin $(RAMDISK)/sbin $(RAMDISK)/etc
	cp $(BUILD)/init $(RAMDISK)/init
	chmod 0755 $(RAMDISK)/init
	cp $(BUSYBOX) $(RAMDISK)/bin/busybox
	chmod 0755 $(RAMDISK)/bin/busybox
	ln -s busybox $(RAMDISK)/bin/sh
	ln -s busybox $(RAMDISK)/bin/ash
	ln -s busybox $(RAMDISK)/bin/ls
	ln -s busybox $(RAMDISK)/bin/cat
	ln -s busybox $(RAMDISK)/bin/dmesg
	ln -s busybox $(RAMDISK)/bin/ps
	ln -s busybox $(RAMDISK)/bin/mount
	ln -s busybox $(RAMDISK)/bin/umount
	ln -s busybox $(RAMDISK)/bin/uname
	ln -s busybox $(RAMDISK)/bin/free
	ln -s busybox $(RAMDISK)/bin/df
	ln -s busybox $(RAMDISK)/bin/sleep
	ln -s busybox $(RAMDISK)/bin/ifconfig
	ln -s busybox $(RAMDISK)/sbin/telnetd
	ln -s busybox $(RAMDISK)/sbin/udhcpd
	ln -s busybox $(RAMDISK)/sbin/httpd
	mkdir -p $(RAMDISK)/dev/pts
	cp os/init/rcS $(RAMDISK)/etc/rcS
	chmod 0755 $(RAMDISK)/etc/rcS
	printf '%s\n' 'root:x:0:0:root:/:/bin/sh' > $(RAMDISK)/etc/passwd
	printf '%s\n' 'root:x:0:' > $(RAMDISK)/etc/group
	( cd $(RAMDISK) && find . | cpio -o -H newc --quiet ) | gzip -n > $(BUILD)/ramdisk.cpio.gz
	@echo "ramdisk $$(wc -c < $(BUILD)/ramdisk.cpio.gz) bytes gzipped"

boot.img: unpack ramdisk
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT)
	sha256sum $(OUT) $(STOCK)/boot.img
	@echo "built $(OUT) — not flashed"

vbmeta: check-stock
	python3 os/bootimg/vbmeta.py --patch-stock $(STOCK)/vbmeta.img -o $(VBMETA_OUT)
	sha256sum $(VBMETA_OUT) $(STOCK)/vbmeta.img

odin-tar: boot.img vbmeta
	rm -rf $(BUILD)/odin-v011 $(BUILD)/odin-stock-restore
	mkdir -p $(BUILD)/odin-v011 $(BUILD)/odin-stock-restore
	cp $(OUT) $(BUILD)/odin-v011/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v011/vbmeta.img
	cp $(STOCK)/boot.img $(BUILD)/odin-stock-restore/boot.img
	cp $(STOCK)/vbmeta.img $(BUILD)/odin-stock-restore/vbmeta.img
	tar -C $(BUILD)/odin-v011 -cf $(BUILD)/saaios-boot-v011.tar boot.img vbmeta.img
	tar -C $(BUILD)/odin-stock-restore -cf $(BUILD)/saaios-boot-stock-restore.tar boot.img vbmeta.img
	@echo "Odin AP tars (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v011.tar"
	@echo "  $(BUILD)/saaios-boot-stock-restore.tar"
	tar -tvf $(BUILD)/saaios-boot-v011.tar
	tar -tvf $(BUILD)/saaios-boot-stock-restore.tar

restore: check-stock
	@echo "REFUSING automatic flash."
	@echo "Restore images:"
	@echo "  $(STOCK)/boot.img"
	@echo "  $(STOCK)/vbmeta.img"
	@echo "Put the phone in Download mode, then Odin AP=boot.img (tar) or Heimdall --BOOT/--VBMETA."
	@exit 2

# PARAM/UP_PARAM only (unlock-warning JPEGs). Not boot-v007. Never flashes.
up_param:
	python3 os/param/up_param.py build --stock $(STOCK) --bl-zip $(BL_ZIP) --out $(BUILD) --media $(MEDIA)

up_param-flash:
	python3 os/param/up_param.py flash

flash: check-stock
	@echo "REFUSING: will not flash from make."
	@echo "Human: Download mode, flash only $(OUT) to BOOT after a photo of OEM LOCK=OFF."
	@echo "Rollback: make restore images in $(STOCK)"
	@echo "PARAM nologo is a separate Odin BL tar; make up_param-flash also refuses."
	@exit 2
