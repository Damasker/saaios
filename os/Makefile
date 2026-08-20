# SaaiOS native boot (Galaxy A12 SM-A127F first target)
# Host-only until you explicitly flash from Download mode.

STOCK ?= os/images/stock/A127FXXSDDXJ2
BUILD := os/build
STOCK_UNPACK := $(BUILD)/stock-boot
RAMDISK := $(BUILD)/ramdisk
OUT := $(BUILD)/boot-v011.img
OUT_V012 := $(BUILD)/boot-v012.img
OUT_V013 := $(BUILD)/boot-v013.img
OUT_V014 := $(BUILD)/boot-v014.img
OUT_V015 := $(BUILD)/boot-v015.img
OUT_V016 := $(BUILD)/boot-v016.img
OUT_V017 := $(BUILD)/boot-v017.img
OUT_V018 := $(BUILD)/boot-v018.img
OUT_V019 := $(BUILD)/boot-v019.img
# Subversion suffix embedded into ramdisk console + used for Odin/AP tar naming.
# Example: since6 → saaios-boot-v019-since6.tar
SUBVERSION_V019 ?= since44
VBMETA_OUT := $(BUILD)/vbmeta-patched.img
BUSYBOX ?= os/third_party/busybox-aarch64
BUSYBOX_DEB_URL ?= https://deb.debian.org/debian/pool/main/b/busybox/busybox-static_1.37.0-6+b9_arm64.deb
DROPBEAR ?= os/third_party/dropbear-aarch64
DROPBEAR_SCP ?= os/third_party/dropbear-scp-aarch64
SFTP_SERVER ?= os/third_party/sftp-server-aarch64
DROPBEAR_REL ?= https://github.com/sagemathinc/dropbear/releases/download/main
DROPBEAR_AUTH_KEYS ?= os/init/ssh/authorized_keys
CROSS ?= aarch64-linux-gnu-
KERNEL_DIR := os/third_party/kernel_samsung_a12
KERNEL_OUT := $(BUILD)/kernel-out
KERNEL_IMAGE ?= $(BUILD)/kernel-Image
KERNEL_PATCH := os/patches/syna-tcm-resume-hdl-idle.patch
KERNEL_PATCH_V013 := os/patches/syna-tcm-fw-sel-td4150.patch
KERNEL_PATCH_V014 := os/patches/syna-tcm-onchip-fw-touch.patch
KERNEL_PATCH_V015 := os/patches/syna-tcm-skip-second-identify.patch
KERNEL_PATCH_V016 := os/patches/syna-tcm-first-appinfo-resync.patch
KERNEL_PATCH_V017 := os/patches/syna-tcm-drain-not-at-probe.patch
KERNEL_PATCH_V018 := os/patches/syna-tcm-enable-report-salvage.patch
KERNEL_PATCH_V019 := os/patches/syna-tcm-attn-before-enable-report.patch
KERNEL_PATCH_SINCE8 := os/patches/syna-tcm-continued-read-chunk.patch
KERNEL_PATCH_SINCE9 := os/patches/syna-tcm-continued-read-complete.patch
KERNEL_PATCH_SINCE10 := os/patches/syna-tcm-enable-report-plen-clear.patch
KERNEL_PATCH_SINCE11 := os/patches/syna-tcm-read-message-plen-zero.patch
KERNEL_PATCH_SINCE12 := os/patches/syna-tcm-drained-status-ok-only.patch
KERNEL_PATCH_SINCE13 := os/patches/syna-tcm-enable-report-empty-ack.patch
KERNEL_PATCH_SINCE14 := os/patches/syna-tcm-probe-enter-log.patch
KERNEL_PATCH_SINCE15 := os/patches/syna-tcm-skip-hdl-identify.patch
KERNEL_PATCH_SINCE16 := os/patches/syna-tcm-stop-poll-dump.patch
KERNEL_PATCH_SINCE17 := os/patches/syna-tcm-enable-report-oneshot-drain.patch
KERNEL_PATCH_SINCE18 := os/patches/syna-tcm-onchip-clear-hdl-reinit.patch
KERNEL_PATCH_SINCE19 := os/patches/syna-tcm-hdl-status-config.patch
KERNEL_PATCH_SINCE20 := os/patches/syna-tcm-sync-download-config.patch
KERNEL_PATCH_SINCE21 := os/patches/syna-tcm-hdl-complete-reinit.patch
KERNEL_PATCH_SINCE22 := os/patches/syna-tcm-hdl-send-0x30.patch
KERNEL_PATCH_SINCE23 := os/patches/syna-tcm-romboot-chunk-0x45.patch
KERNEL_PATCH_SINCE24 := os/patches/syna-tcm-hdl-status-idle.patch
KERNEL_PATCH_SINCE25 := os/patches/syna-tcm-romboot-drain-attn.patch
KERNEL_PATCH_SINCE26 := os/patches/syna-tcm-romboot-hold-irq.patch
KERNEL_PATCH_SINCE27 := os/patches/syna-tcm-romboot-0x1b-chunk.patch
KERNEL_PATCH_SINCE28 := os/patches/syna-tcm-romboot-remaining.patch
KERNEL_PATCH_SINCE29 := os/patches/syna-tcm-romboot-per-chunk-u16.patch
KERNEL_PATCH_SINCE30 := os/patches/syna-tcm-romboot-stock-continue.patch
KERNEL_PATCH_SINCE31 := os/patches/syna-tcm-romboot-after-send-drain.patch
KERNEL_PATCH_SINCE32 := os/patches/syna-tcm-romboot-stock-post-0x45.patch
KERNEL_PATCH_SINCE33 := os/patches/syna-tcm-romboot-0x1b-complete-waiter.patch
KERNEL_PATCH_SINCE34 := os/patches/syna-tcm-romboot-identify-after-42.patch
KERNEL_PATCH_SINCE35 := os/patches/syna-tcm-romboot-fw-areas-0x40.patch
KERNEL_PATCH_SINCE36 := os/patches/syna-tcm-romboot-0x40-status-ok.patch
KERNEL_PATCH_SINCE37 := os/patches/syna-tcm-romboot-panel-defer.patch
KERNEL_PATCH_SINCE38 := os/patches/syna-tcm-romboot-probe-zero-045.patch
KERNEL_PATCH_SINCE39 := os/patches/syna-tcm-romboot-0x45-oneshot.patch
KERNEL_PATCH_SINCE40 := os/patches/syna-tcm-romboot-bringup-after-02.patch
KERNEL_PATCH_SINCE41 := os/patches/syna-tcm-romboot-0x02-app-config.patch
KERNEL_PATCH_SINCE42 := os/patches/syna-tcm-romboot-0x02-irq-live.patch
KERNEL_PATCH_SINCE43 := os/patches/syna-tcm-romboot-0x30-irq-live.patch
KERNEL_PATCH_SINCE44 := os/patches/syna-tcm-romboot-0x30-oneshot.patch
KERNEL_PATCHES := $(KERNEL_PATCH) $(KERNEL_PATCH_V013) $(KERNEL_PATCH_V014) $(KERNEL_PATCH_V015) $(KERNEL_PATCH_V016) $(KERNEL_PATCH_V017) $(KERNEL_PATCH_V018) $(KERNEL_PATCH_V019) $(KERNEL_PATCH_SINCE8) $(KERNEL_PATCH_SINCE9) $(KERNEL_PATCH_SINCE10) $(KERNEL_PATCH_SINCE11) $(KERNEL_PATCH_SINCE12) $(KERNEL_PATCH_SINCE13) $(KERNEL_PATCH_SINCE14) $(KERNEL_PATCH_SINCE15) $(KERNEL_PATCH_SINCE16) $(KERNEL_PATCH_SINCE17) $(KERNEL_PATCH_SINCE18) $(KERNEL_PATCH_SINCE19) $(KERNEL_PATCH_SINCE20) $(KERNEL_PATCH_SINCE21) $(KERNEL_PATCH_SINCE22) $(KERNEL_PATCH_SINCE23) $(KERNEL_PATCH_SINCE24) $(KERNEL_PATCH_SINCE25) $(KERNEL_PATCH_SINCE26) $(KERNEL_PATCH_SINCE27) $(KERNEL_PATCH_SINCE28) $(KERNEL_PATCH_SINCE29) $(KERNEL_PATCH_SINCE30) $(KERNEL_PATCH_SINCE31) $(KERNEL_PATCH_SINCE32) $(KERNEL_PATCH_SINCE33) $(KERNEL_PATCH_SINCE34) $(KERNEL_PATCH_SINCE35) $(KERNEL_PATCH_SINCE36) $(KERNEL_PATCH_SINCE37) $(KERNEL_PATCH_SINCE38) $(KERNEL_PATCH_SINCE39) $(KERNEL_PATCH_SINCE40) $(KERNEL_PATCH_SINCE41) $(KERNEL_PATCH_SINCE42) $(KERNEL_PATCH_SINCE43) $(KERNEL_PATCH_SINCE44)
KERNEL_DEFCONFIG := exynos850-a12snsxx_defconfig
# 4.19 wants clang 9 (AOSP r353983c family). clang-17 on R620 is too new.
CLANG_BIN ?= os/third_party/clang-9/bin
# BOOT partition on SM-A127F is 44 MiB; stock boot.img is padded to this with AVB.
BOOT_PART_SIZE := 46137344
BL_ZIP ?= /srv/media/BL_A127FXXSDDXJ2_QB86601342_REV00_user_low_ship_MULTI_CERT.tar.md5.zip
MEDIA ?= /srv/media

.PHONY: all unpack init ramdisk boot.img vbmeta odin-tar restore flash check-stock busybox dropbear \
	up_param up_param-flash boot-v012 boot-v013 boot-v014 boot-v015 boot-v016 boot-v017 boot-v018 boot-v019 kernel kernel-patch

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

$(DROPBEAR) $(DROPBEAR_SCP) $(SFTP_SERVER): os/third_party/dropbear-aarch64.sha256
	mkdir -p os/third_party $(BUILD)/dropbear-dl
	wget -q -O $(BUILD)/dropbear-dl/dropbear-aarch64-linux-musl.tar.xz \
		"$(DROPBEAR_REL)/dropbear-aarch64-linux-musl.tar.xz"
	wget -q -O $(BUILD)/dropbear-dl/sftp-server-aarch64-linux-musl.tar.xz \
		"$(DROPBEAR_REL)/sftp-server-aarch64-linux-musl.tar.xz"
	cd $(BUILD)/dropbear-dl && sha256sum -c $(CURDIR)/os/third_party/dropbear-aarch64.sha256
	tar -C $(BUILD)/dropbear-dl -xf $(BUILD)/dropbear-dl/dropbear-aarch64-linux-musl.tar.xz
	tar -C $(BUILD)/dropbear-dl -xf $(BUILD)/dropbear-dl/sftp-server-aarch64-linux-musl.tar.xz
	cp $(BUILD)/dropbear-dl/dropbear-aarch64-linux-musl/dropbear $(DROPBEAR)
	cp $(BUILD)/dropbear-dl/dropbear-aarch64-linux-musl/scp $(DROPBEAR_SCP)
	cp $(BUILD)/dropbear-dl/sftp-server-aarch64-linux-musl/sftp-server $(SFTP_SERVER)
	chmod 0755 $(DROPBEAR) $(DROPBEAR_SCP) $(SFTP_SERVER)
	aarch64-linux-gnu-strip --strip-unneeded $(SFTP_SERVER) || true

dropbear: $(DROPBEAR) $(DROPBEAR_SCP) $(SFTP_SERVER)

init: $(BUILD)/init

ramdisk: $(BUILD)/init $(BUSYBOX) $(DROPBEAR) $(DROPBEAR_SCP) $(SFTP_SERVER) $(DROPBEAR_AUTH_KEYS)
	rm -rf $(RAMDISK)
	mkdir -p $(RAMDISK)/dev $(RAMDISK)/proc $(RAMDISK)/sys $(RAMDISK)/tmp $(RAMDISK)/run \
		$(RAMDISK)/bin $(RAMDISK)/sbin $(RAMDISK)/etc/dropbear $(RAMDISK)/root \
		$(RAMDISK)/usr/libexec $(RAMDISK)/dev/pts
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
	ln -s busybox $(RAMDISK)/bin/wget
	ln -s busybox $(RAMDISK)/bin/nc
	ln -s busybox $(RAMDISK)/bin/hexdump
	ln -s busybox $(RAMDISK)/bin/grep
	ln -s busybox $(RAMDISK)/bin/mkdir
	ln -s busybox $(RAMDISK)/bin/chmod
	ln -s busybox $(RAMDISK)/bin/kill
	ln -s busybox $(RAMDISK)/bin/head
	ln -s busybox $(RAMDISK)/bin/tail
	ln -s busybox $(RAMDISK)/sbin/telnetd
	ln -s busybox $(RAMDISK)/sbin/udhcpd
	cp $(DROPBEAR) $(RAMDISK)/sbin/dropbear
	cp $(DROPBEAR_SCP) $(RAMDISK)/bin/scp
	cp $(SFTP_SERVER) $(RAMDISK)/usr/libexec/sftp-server
	chmod 0755 $(RAMDISK)/sbin/dropbear $(RAMDISK)/bin/scp $(RAMDISK)/usr/libexec/sftp-server
	cp $(DROPBEAR_AUTH_KEYS) $(RAMDISK)/etc/dropbear/authorized_keys
	cp os/init/shells $(RAMDISK)/etc/shells
	chmod 0644 $(RAMDISK)/etc/shells
	chmod 0700 $(RAMDISK)/etc/dropbear
	chmod 0600 $(RAMDISK)/etc/dropbear/authorized_keys
	cp os/init/rcS $(RAMDISK)/etc/rcS
	chmod 0755 $(RAMDISK)/etc/rcS
	printf '%s\n' 'root:x:0:0:root:/root:/bin/sh' > $(RAMDISK)/etc/passwd
	printf '%s\n' 'root:x:0:' > $(RAMDISK)/etc/group
	( cd $(RAMDISK) && find . | cpio -o -H newc --owner 0:0 --quiet ) | gzip -n > $(BUILD)/ramdisk.cpio.gz
	@echo "ramdisk $$(wc -c < $(BUILD)/ramdisk.cpio.gz) bytes gzipped"

boot.img: unpack ramdisk
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT)
	sha256sum $(OUT) $(STOCK)/boot.img
	@echo "built $(OUT) — not flashed"

kernel-patch:
	@test -f $(KERNEL_DIR)/Makefile || { echo "missing $(KERNEL_DIR) (gitignored clone)"; exit 1; }
	@for p in $(KERNEL_PATCHES); do \
		case $$p in \
			*hdl-idle*) m='in_hdl_mode && atomic_read(&tcm_hcd->host_downloading)' ;; \
			*fw-sel*) m='lcdtype mismatch, using idx' ;; \
			*onchip-fw-touch*) m='skip boot factory rawdata' ;; \
			*skip-second-identify*) m='On-chip firmware, skip second identify' ;; \
			*first-appinfo-resync*) m='ROMBOOT DOWNLOAD skipped, firmware mode' ;; \
			*drain-not-at-probe*) m='v017: no SPI drain here; cold-boot marker 0x00 is normal RomBoot' ;; \
			*enable-report-salvage*) m='enabled REPORT_TOUCH (salvaged)' ;; \
			*attn-before-enable-report*) m='ATTN before clear' ;; \
			*continued-read-chunk*) m='syna_tcm_effective_rd_chunk_size' ;; \
			*continued-read-complete*) m='TD4150 sends STATUS_CONTINUED_READ (0x03) on every chunk' ;; \
			*enable-report-plen-clear*) m='syna_tcm_status_ok_drained' ;; \
			*read-message-plen-zero*) m='response_timeout_ms = 3000' ;; \
			*drained-status-ok-only*) m='Only mark transport-complete for STATUS_OK chains' ;; \
			*enable-report-empty-ack*) m='ENABLE_REPORT ignore leftover' ;; \
			*probe-enter-log*) m='SAaiOS_TOUCH_DBG: probe enter' ;; \
			*skip-hdl-identify*) m='GET_APPLICATION_INFO timed out, continuing with on-chip FW' ;; \
			*stop-poll-dump*) m='polling stop repeated leftover dump' ;; \
			*oneshot-drain*) m='ENABLE_REPORT timeout: one-shot leftover drain' ;; \
			*onchip-clear-hdl-reinit*) m='on-chip 0x02, in_hdl_mode=0 (stock clean-0x02)' ;; \
			*hdl-status-config*) m='zeroflash_download_config ran=' ;; \
			*sync-download-config*) m='write_message CMD_DOWNLOAD_CONFIG 0x30' ;; \
			*hdl-complete-reinit*) m='HDL complete helper HELP_SEND_REINIT_NOTIFICATION' ;; \
			*hdl-send-0x30*) m='0x30 payload source=fw_app_config' ;; \
			*romboot-chunk*) m='since23 path=A' ;; \
			*hdl-status-idle*) m='SAaiOS_TOUCH_DBG: since24 probe' ;; \
			*romboot-drain-attn*) m='SAaiOS_TOUCH_DBG: since25 probe' ;; \
			*romboot-hold-irq*) m='SAaiOS_TOUCH_DBG: since26 probe' ;; \
			*romboot-0x1b-chunk*) m='SAaiOS_TOUCH_DBG: since27 probe' ;; \
			*romboot-remaining*) m='SAaiOS_TOUCH_DBG: since28 probe' ;; \
			*romboot-per-chunk-u16*) m='SAaiOS_TOUCH_DBG: since29 probe' ;; \
			*romboot-stock-continue*) m='SAaiOS_TOUCH_DBG: since30 probe' ;; \
			*romboot-after-send-drain*) m='SAaiOS_TOUCH_DBG: since31 probe' ;; \
			*romboot-stock-post-0x45*) m='SAaiOS_TOUCH_DBG: since32 probe' ;; \
			*romboot-0x1b-complete-waiter*) m='SAaiOS_TOUCH_DBG: since33 probe' ;; \
			*romboot-identify-after-42*) m='SAaiOS_TOUCH_DBG: since34 probe' ;; \
			*romboot-fw-areas-0x40*) m='SAaiOS_TOUCH_DBG: since35 probe' ;; \
			*romboot-0x40-status-ok*) m='SAaiOS_TOUCH_DBG: since36 probe' ;; \
			*romboot-panel-defer*) m='SAaiOS_TOUCH_DBG: since37 probe' ;; \
			*romboot-probe-zero-045*) m='SAaiOS_TOUCH_DBG: since38 probe' ;; \
			*romboot-0x45-oneshot*) m='SAaiOS_TOUCH_DBG: since39 probe' ;; \
			*romboot-bringup-after-02*) m='SAaiOS_TOUCH_DBG: since40 probe' ;; \
			*romboot-0x02-app-config*) m='SAaiOS_TOUCH_DBG: since41 probe' ;; \
			*romboot-0x02-irq-live*) m='SAaiOS_TOUCH_DBG: since42 probe' ;; \
			*romboot-0x30-irq-live*) m='SAaiOS_TOUCH_DBG: since43 probe' ;; \
			*romboot-0x30-oneshot*) m='SAaiOS_TOUCH_DBG: since44 probe' ;; \
			*) m='' ;; \
		esac; \
		if [ -n "$$m" ] && grep -Rql -- "$$m" $(KERNEL_DIR)/drivers/input/touchscreen/synaptics/td4150/; then \
			echo "$$p already applied"; \
		elif patch -d $(KERNEL_DIR) -p1 --forward --dry-run --silent < $$p; then \
			patch -d $(KERNEL_DIR) -p1 --forward < $$p; \
		else \
			echo "FAILED to apply $$p"; exit 1; \
		fi; \
	done

# Out-of-tree build; copies Image to $(KERNEL_IMAGE) for boot-v012.
# Exact cmdline (also in docs/os/targets/sm-a127f/kernel-touch.md):
#   PATH=os/third_party/clang-9/bin:$$PATH PLATFORM_VERSION=13 ANDROID_MAJOR_VERSION=t ARCH=arm64 \
#   make O=os/build/kernel-out ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- \
#     CLANG_TRIPLE=aarch64-linux-gnu- PYTHON=python3 PYTHON2=python3 PYTHON3=python3 \
#     exynos850-a12snsxx_defconfig && make O=... -j$$(nproc)
kernel: kernel-patch
	@test -x $(CLANG_BIN)/clang || { echo "missing $(CLANG_BIN)/clang — extract LLVM 9.0.1 into os/third_party/clang-9"; exit 1; }
	mkdir -p $(KERNEL_OUT)
	cd $(KERNEL_DIR) && \
		PATH="$(CURDIR)/$(CLANG_BIN):$$PATH" \
		PLATFORM_VERSION=13 ANDROID_MAJOR_VERSION=t ARCH=arm64 \
		$(MAKE) O=$(CURDIR)/$(KERNEL_OUT) ARCH=arm64 \
			CROSS_COMPILE=aarch64-linux-gnu- CLANG_TRIPLE=aarch64-linux-gnu- \
			PYTHON=python3 PYTHON2=python3 PYTHON3=python3 \
			$(KERNEL_DEFCONFIG)
	# Project-Xed rtl8188eus fails clang-9 fallthrough (-Werror); USB dongle, not TSP.
	# See docs/os/targets/sm-a127f/wifi-usb.md — do not enable for onboard wlan.
	$(KERNEL_DIR)/scripts/config --file $(KERNEL_OUT)/.config -d CONFIG_RTL8188EU
	cd $(KERNEL_DIR) && \
		PATH="$(CURDIR)/$(CLANG_BIN):$$PATH" \
		PLATFORM_VERSION=13 ANDROID_MAJOR_VERSION=t ARCH=arm64 \
		$(MAKE) O=$(CURDIR)/$(KERNEL_OUT) ARCH=arm64 \
			CROSS_COMPILE=aarch64-linux-gnu- CLANG_TRIPLE=aarch64-linux-gnu- \
			PYTHON=python3 PYTHON2=python3 PYTHON3=python3 \
			-j$$(nproc)
	@test -f $(KERNEL_OUT)/arch/arm64/boot/Image || { echo "kernel build produced no Image"; exit 1; }
	cp -f $(KERNEL_OUT)/arch/arm64/boot/Image $(KERNEL_IMAGE)
	@echo "built $(KERNEL_IMAGE) — not packed, not flashed. Run: make -f os/Makefile boot-v019"

# boot-v012: stock DTB (from unpacked stock boot.img) + new Image if present.
# Does not flash. Copies $(MEDIA)/saaios-boot-v012.tar only when Image exists.
ifeq ($(wildcard $(KERNEL_IMAGE)),)
boot-v012:
	@echo "boot-v012: Image missing at $(KERNEL_IMAGE)"
	@echo "Apply $(KERNEL_PATCH), then build with exynos850-a12snsxx_defconfig."
	@echo "See docs/os/targets/sm-a127f/kernel-touch.md."
	@echo "Not packing. Not copying $(MEDIA)/saaios-boot-v012.tar."
	@echo "Phone stays on v011. make flash still refuses."
else
boot-v012: unpack ramdisk vbmeta
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--kernel $(KERNEL_IMAGE) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V012)
	rm -rf $(BUILD)/odin-v012
	mkdir -p $(BUILD)/odin-v012
	cp $(OUT_V012) $(BUILD)/odin-v012/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v012/vbmeta.img
	tar -C $(BUILD)/odin-v012 -cf $(BUILD)/saaios-boot-v012.tar boot.img vbmeta.img
	cp $(BUILD)/saaios-boot-v012.tar $(MEDIA)/saaios-boot-v012.tar
	sha256sum $(OUT_V012) $(KERNEL_IMAGE)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v012.tar"
	@echo "  copied $(MEDIA)/saaios-boot-v012.tar"
	tar -tvf $(BUILD)/saaios-boot-v012.tar
endif

# boot-v013: stock DTB from A127FXXSDDXJ2 boot.img + patched Image + v013 ramdisk.
# Does not flash. Never substitutes Project-Xed exynos3830.dtb.
ifeq ($(wildcard $(KERNEL_IMAGE)),)
boot-v013:
	@echo "boot-v013: Image missing at $(KERNEL_IMAGE)"
	@echo "Apply $(KERNEL_PATCHES), then build with exynos850-a12snsxx_defconfig."
	@echo "See docs/os/targets/sm-a127f/kernel-touch.md."
	@echo "Not packing. Not copying $(MEDIA)/saaios-boot-v013.tar."
	@echo "Phone stays on current boot. make flash still refuses."
else
boot-v013: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--kernel $(KERNEL_IMAGE) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V013)
	python3 os/bootimg/bootimg.py unpack $(OUT_V013) -o $(BUILD)/boot-v013-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v013-check/dtb
	@echo "packed DTB matches stock A127FXXSDDXJ2 (not Project-Xed exynos3830.dtb)"
	rm -rf $(BUILD)/odin-v013
	mkdir -p $(BUILD)/odin-v013
	cp $(OUT_V013) $(BUILD)/odin-v013/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v013/vbmeta.img
	tar -C $(BUILD)/odin-v013 -cf $(BUILD)/saaios-boot-v013.tar boot.img vbmeta.img
	cp $(BUILD)/saaios-boot-v013.tar $(MEDIA)/saaios-boot-v013.tar
	sha256sum $(OUT_V013) $(KERNEL_IMAGE)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v013.tar"
	@echo "  copied $(MEDIA)/saaios-boot-v013.tar"
	tar -tvf $(BUILD)/saaios-boot-v013.tar
endif

# boot-v014: stock DTB from A127FXXSDDXJ2 boot.img + patched Image + v014 ramdisk.
# Does not flash. Never substitutes Project-Xed exynos3830.dtb.
ifeq ($(wildcard $(KERNEL_IMAGE)),)
boot-v014:
	@echo "boot-v014: Image missing at $(KERNEL_IMAGE)"
	@echo "Apply $(KERNEL_PATCHES), then build with exynos850-a12snsxx_defconfig."
	@echo "See docs/os/targets/sm-a127f/kernel-touch.md."
	@echo "Not packing. Not copying $(MEDIA)/saaios-boot-v014.tar."
	@echo "Phone stays on current boot. make flash still refuses."
else
boot-v014: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--kernel $(KERNEL_IMAGE) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V014)
	python3 os/bootimg/bootimg.py unpack $(OUT_V014) -o $(BUILD)/boot-v014-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v014-check/dtb
	@echo "packed DTB matches stock A127FXXSDDXJ2 (not Project-Xed exynos3830.dtb)"
	rm -rf $(BUILD)/odin-v014
	mkdir -p $(BUILD)/odin-v014
	cp $(OUT_V014) $(BUILD)/odin-v014/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v014/vbmeta.img
	tar -C $(BUILD)/odin-v014 -cf $(BUILD)/saaios-boot-v014.tar boot.img vbmeta.img
	cp $(BUILD)/saaios-boot-v014.tar $(MEDIA)/saaios-boot-v014.tar
	sha256sum $(OUT_V014) $(KERNEL_IMAGE)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v014.tar"
	@echo "  copied $(MEDIA)/saaios-boot-v014.tar"
	tar -tvf $(BUILD)/saaios-boot-v014.tar
endif

# boot-v015: stock DTB from A127FXXSDDXJ2 boot.img + patched Image + v015 ramdisk.
# Does not flash. Never substitutes Project-Xed exynos3830.dtb.
ifeq ($(wildcard $(KERNEL_IMAGE)),)
boot-v015:
	@echo "boot-v015: Image missing at $(KERNEL_IMAGE)"
	@echo "Apply $(KERNEL_PATCHES), then build with exynos850-a12snsxx_defconfig."
	@echo "See docs/os/targets/sm-a127f/kernel-touch.md."
	@echo "Not packing. Not copying $(MEDIA)/saaios-boot-v015.tar."
	@echo "Phone stays on current boot. make flash still refuses."
else
boot-v015: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--kernel $(KERNEL_IMAGE) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V015)
	python3 os/bootimg/bootimg.py unpack $(OUT_V015) -o $(BUILD)/boot-v015-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v015-check/dtb
	@echo "packed DTB matches stock A127FXXSDDXJ2 (not Project-Xed exynos3830.dtb)"
	rm -rf $(BUILD)/odin-v015
	mkdir -p $(BUILD)/odin-v015
	cp $(OUT_V015) $(BUILD)/odin-v015/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v015/vbmeta.img
	tar -C $(BUILD)/odin-v015 -cf $(BUILD)/saaios-boot-v015.tar boot.img vbmeta.img
	cp $(BUILD)/saaios-boot-v015.tar $(MEDIA)/saaios-boot-v015.tar
	sha256sum $(OUT_V015) $(KERNEL_IMAGE)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v015.tar"
	@echo "  copied $(MEDIA)/saaios-boot-v015.tar"
	tar -tvf $(BUILD)/saaios-boot-v015.tar
endif

# boot-v016: stock DTB from A127FXXSDDXJ2 boot.img + patched Image + v016 ramdisk.
# Does not flash. Never substitutes Project-Xed exynos3830.dtb.
ifeq ($(wildcard $(KERNEL_IMAGE)),)
boot-v016:
	@echo "boot-v016: Image missing at $(KERNEL_IMAGE)"
	@echo "Apply $(KERNEL_PATCHES), then build with exynos850-a12snsxx_defconfig."
	@echo "See docs/os/targets/sm-a127f/kernel-touch.md."
	@echo "Not packing. Not copying $(MEDIA)/saaios-boot-v016.tar."
	@echo "Phone stays on current boot. make flash still refuses."
else
boot-v016: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--kernel $(KERNEL_IMAGE) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V016)
	python3 os/bootimg/bootimg.py unpack $(OUT_V016) -o $(BUILD)/boot-v016-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v016-check/dtb
	@echo "packed DTB matches stock A127FXXSDDXJ2 (not Project-Xed exynos3830.dtb)"
	rm -rf $(BUILD)/odin-v016
	mkdir -p $(BUILD)/odin-v016
	cp $(OUT_V016) $(BUILD)/odin-v016/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v016/vbmeta.img
	tar -C $(BUILD)/odin-v016 -cf $(BUILD)/saaios-boot-v016.tar boot.img vbmeta.img
	cp $(BUILD)/saaios-boot-v016.tar $(MEDIA)/saaios-boot-v016.tar
	sha256sum $(OUT_V016) $(KERNEL_IMAGE)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v016.tar"
	@echo "  copied $(MEDIA)/saaios-boot-v016.tar"
	tar -tvf $(BUILD)/saaios-boot-v016.tar
endif

# boot-v017: stock DTB from A127FXXSDDXJ2 boot.img + patched Image + v017 ramdisk.
# Does not flash. Never substitutes Project-Xed exynos3830.dtb.
ifeq ($(wildcard $(KERNEL_IMAGE)),)
boot-v017:
	@echo "boot-v017: Image missing at $(KERNEL_IMAGE)"
	@echo "Apply $(KERNEL_PATCHES), then build with exynos850-a12snsxx_defconfig."
	@echo "See docs/os/targets/sm-a127f/kernel-touch.md."
	@echo "Not packing. Not copying $(MEDIA)/saaios-boot-v017.tar."
	@echo "Phone stays on current boot. make flash still refuses."
else
boot-v017: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--kernel $(KERNEL_IMAGE) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V017)
	python3 os/bootimg/bootimg.py unpack $(OUT_V017) -o $(BUILD)/boot-v017-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v017-check/dtb
	@echo "packed DTB matches stock A127FXXSDDXJ2 (not Project-Xed exynos3830.dtb)"
	rm -rf $(BUILD)/odin-v017
	mkdir -p $(BUILD)/odin-v017
	cp $(OUT_V017) $(BUILD)/odin-v017/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v017/vbmeta.img
	tar -C $(BUILD)/odin-v017 -cf $(BUILD)/saaios-boot-v017.tar boot.img vbmeta.img
	cp $(BUILD)/saaios-boot-v017.tar $(MEDIA)/saaios-boot-v017.tar
	sha256sum $(OUT_V017) $(KERNEL_IMAGE)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v017.tar"
	@echo "  copied $(MEDIA)/saaios-boot-v017.tar"
	tar -tvf $(BUILD)/saaios-boot-v017.tar
endif

# boot-v018: stock DTB from A127FXXSDDXJ2 boot.img + patched Image + v018 ramdisk.
# Does not flash. Never substitutes Project-Xed exynos3830.dtb.
ifeq ($(wildcard $(KERNEL_IMAGE)),)
boot-v018:
	@echo "boot-v018: Image missing at $(KERNEL_IMAGE)"
	@echo "Apply $(KERNEL_PATCHES), then build with exynos850-a12snsxx_defconfig."
	@echo "See docs/os/targets/sm-a127f/kernel-touch.md."
	@echo "Not packing. Not copying $(MEDIA)/saaios-boot-v018.tar."
	@echo "Phone stays on current boot. make flash still refuses."
else
boot-v018: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--kernel $(KERNEL_IMAGE) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V018)
	python3 os/bootimg/bootimg.py unpack $(OUT_V018) -o $(BUILD)/boot-v018-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v018-check/dtb
	@echo "packed DTB matches stock A127FXXSDDXJ2 (not Project-Xed exynos3830.dtb)"
	rm -rf $(BUILD)/odin-v018
	mkdir -p $(BUILD)/odin-v018
	cp $(OUT_V018) $(BUILD)/odin-v018/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v018/vbmeta.img
	tar -C $(BUILD)/odin-v018 -cf $(BUILD)/saaios-boot-v018.tar boot.img vbmeta.img
	cp $(BUILD)/saaios-boot-v018.tar $(MEDIA)/saaios-boot-v018.tar
	sha256sum $(OUT_V018) $(KERNEL_IMAGE)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v018.tar"
	@echo "  copied $(MEDIA)/saaios-boot-v018.tar"
	tar -tvf $(BUILD)/saaios-boot-v018.tar
endif

# boot-v019: stock DTB + v019 kernel (ATTN clear, skip 0x25) + v019 ramdisk.
ifeq ($(wildcard $(KERNEL_IMAGE)),)
boot-v019:
	@echo "boot-v019: Image missing at $(KERNEL_IMAGE)"
	@echo "Apply $(KERNEL_PATCHES), then build with exynos850-a12snsxx_defconfig."
	@echo "See docs/os/targets/sm-a127f/kernel-touch.md."
	@echo "Not packing. Not copying $(MEDIA)/saaios-boot-v019-$(SUBVERSION_V019).tar."
	@echo "Phone stays on current boot. make flash still refuses."
else
boot-v019: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--kernel $(KERNEL_IMAGE) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V019)
	python3 os/bootimg/bootimg.py unpack $(OUT_V019) -o $(BUILD)/boot-v019-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v019-check/dtb
	@echo "packed DTB matches stock A127FXXSDDXJ2 (not Project-Xed exynos3830.dtb)"
	rm -rf $(BUILD)/odin-v019
	mkdir -p $(BUILD)/odin-v019
	cp $(OUT_V019) $(BUILD)/odin-v019/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v019/vbmeta.img
	tar -C $(BUILD)/odin-v019 -cf $(BUILD)/saaios-boot-v019-$(SUBVERSION_V019).tar boot.img vbmeta.img
	cp $(BUILD)/saaios-boot-v019-$(SUBVERSION_V019).tar $(MEDIA)/saaios-boot-v019-$(SUBVERSION_V019).tar
	chmod 664 $(MEDIA)/saaios-boot-v019-$(SUBVERSION_V019).tar
	-chown mike:media $(MEDIA)/saaios-boot-v019-$(SUBVERSION_V019).tar
	sha256sum $(OUT_V019) $(KERNEL_IMAGE)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v019-$(SUBVERSION_V019).tar"
	@echo "  copied $(MEDIA)/saaios-boot-v019-$(SUBVERSION_V019).tar"
	tar -tvf $(BUILD)/saaios-boot-v019-$(SUBVERSION_V019).tar
endif

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
	@echo "Human: Download mode, flash only a packed boot.img to BOOT after a photo of OEM LOCK=OFF."
	@echo "  v011: $(OUT)"
	@echo "  v012: $(OUT_V012) (only if $(KERNEL_IMAGE) was packed)"
	@echo "  v013: $(OUT_V013) (only if $(KERNEL_IMAGE) was packed)"
	@echo "  v014: $(OUT_V014) (only if $(KERNEL_IMAGE) was packed)"
	@echo "  v015: $(OUT_V015) (only if $(KERNEL_IMAGE) was packed)"
	@echo "  v016: $(OUT_V016) (only if $(KERNEL_IMAGE) was packed)"
	@echo "  v017: $(OUT_V017) (only if $(KERNEL_IMAGE) was packed)"
	@echo "  v018: $(OUT_V018) (only if $(KERNEL_IMAGE) was packed)"
	@echo "Rollback: make restore images in $(STOCK)"
	@echo "PARAM nologo is a separate Odin BL tar; make up_param-flash also refuses."
	@exit 2
