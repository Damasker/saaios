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
OUT_V020 := $(BUILD)/boot-v020.img
OUT_V021 := $(BUILD)/boot-v021.img
OUT_V022 := $(BUILD)/boot-v022.img
OUT_V023 := $(BUILD)/boot-v023.img
OUT_V024 := $(BUILD)/boot-v024.img
OUT_V025 := $(BUILD)/boot-v025.img
OUT_V026 := $(BUILD)/boot-v026.img
OUT_V027 := $(BUILD)/boot-v027.img
OUT_V028 := $(BUILD)/boot-v028.img
OUT_V029 := $(BUILD)/boot-v029.img
OUT_V030 := $(BUILD)/boot-v030.img
OUT_V031 := $(BUILD)/boot-v031.img
# Subversion suffix embedded into ramdisk console + used for Odin/AP tar naming.
# since76 = live20 ladder 0/10/100/500ms after REINIT; retval<0→dead response=ff.
# Late 0x05/0x24/0x30 LIVE -62. No app_config in post-boot menu. Do not overwrite since54–since75.
# See docs/os/targets/sm-a127f/kernel-touch.md.
SUBVERSION_V019 ?= since76
VBMETA_OUT := $(BUILD)/vbmeta-patched.img
BUSYBOX ?= os/third_party/busybox-aarch64
BUSYBOX_DEB_URL ?= https://deb.debian.org/debian/pool/main/b/busybox/busybox-static_1.37.0-6+b9_arm64.deb
DROPBEAR ?= os/third_party/dropbear-aarch64
DROPBEAR_SCP ?= os/third_party/dropbear-scp-aarch64
SFTP_SERVER ?= os/third_party/sftp-server-aarch64
IW ?= os/third_party/iw-aarch64
WPA_SUPPLICANT ?= os/third_party/wpa_supplicant-aarch64
WPA_CLI ?= os/third_party/wpa_cli-aarch64
DROPBEAR_REL ?= https://github.com/sagemathinc/dropbear/releases/download/main
DROPBEAR_AUTH_KEYS ?= os/init/ssh/authorized_keys
TINYALSA := os/third_party/tinyalsa
VENDOR_IMG ?= os/build/stock-super/vendor.img
DEBUGFS ?= /sbin/debugfs
ABOX_FW := $(BUILD)/abox-fw
ABOX_FW_BINS := calliope_sram.bin calliope_sram_2.bin calliope_dram.bin calliope_dram_2.bin \
	AP_AUDIO_SLSI.bin APDV_AUDIO_SLSI.bin abox_tplg.bin
WIFI_FW := $(BUILD)/wifi-fw
# Production Maxwell (skip mx140_t.bin / mx140_t_* / mx140/debug).
WIFI_FW_FILES := mx140.bin mx140_bt.hcf mx140_common.hcf mx140_wlan.hcf \
	mx140_wlan_sw.hcf mx140_wlan_t.hcf mx140_wlan_t_sw.hcf \
	hydra_config.sdb slsi_reg_database.bin platform.txt \
	indoorchannel.info wpa_supplicant.conf wpa_supplicant_overlay.conf \
	p2p_supplicant_overlay.conf
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
KERNEL_PATCH_SINCE45 := os/patches/syna-tcm-romboot-skip-0x30-reinit.patch
KERNEL_PATCH_SINCE46 := os/patches/syna-tcm-romboot-identify-dump.patch
KERNEL_PATCH_SINCE47 := os/patches/syna-tcm-romboot-app-code-id.patch
KERNEL_PATCH_SINCE48 := os/patches/syna-tcm-romboot-0x42-after-02.patch
KERNEL_PATCH_SINCE49 := os/patches/syna-tcm-romboot-identify-before-after.patch
KERNEL_PATCH_SINCE50 := os/patches/syna-tcm-romboot-hdl-1b-trace.patch
KERNEL_PATCH_SINCE51 := os/patches/syna-tcm-romboot-hdl-serialize.patch
KERNEL_PATCH_SINCE52 := os/patches/syna-tcm-romboot-hdl-switch-bootloader.patch
KERNEL_PATCH_SINCE53 := os/patches/syna-tcm-romboot-hdl-0x42-from-02.patch
KERNEL_PATCH_SINCE54 := os/patches/syna-tcm-romboot-hdl-no-post-cmd.patch
KERNEL_PATCHES := $(KERNEL_PATCH) $(KERNEL_PATCH_V013) $(KERNEL_PATCH_V014) $(KERNEL_PATCH_V015) $(KERNEL_PATCH_V016) $(KERNEL_PATCH_V017) $(KERNEL_PATCH_V018) $(KERNEL_PATCH_V019) $(KERNEL_PATCH_SINCE8) $(KERNEL_PATCH_SINCE9) $(KERNEL_PATCH_SINCE10) $(KERNEL_PATCH_SINCE11) $(KERNEL_PATCH_SINCE12) $(KERNEL_PATCH_SINCE13) $(KERNEL_PATCH_SINCE14) $(KERNEL_PATCH_SINCE15) $(KERNEL_PATCH_SINCE16) $(KERNEL_PATCH_SINCE17) $(KERNEL_PATCH_SINCE18) $(KERNEL_PATCH_SINCE19) $(KERNEL_PATCH_SINCE20) $(KERNEL_PATCH_SINCE21) $(KERNEL_PATCH_SINCE22) $(KERNEL_PATCH_SINCE23) $(KERNEL_PATCH_SINCE24) $(KERNEL_PATCH_SINCE25) $(KERNEL_PATCH_SINCE26) $(KERNEL_PATCH_SINCE27) $(KERNEL_PATCH_SINCE28) $(KERNEL_PATCH_SINCE29) $(KERNEL_PATCH_SINCE30) $(KERNEL_PATCH_SINCE31) $(KERNEL_PATCH_SINCE32) $(KERNEL_PATCH_SINCE33) $(KERNEL_PATCH_SINCE34) $(KERNEL_PATCH_SINCE35) $(KERNEL_PATCH_SINCE36) $(KERNEL_PATCH_SINCE37) $(KERNEL_PATCH_SINCE38) $(KERNEL_PATCH_SINCE39) $(KERNEL_PATCH_SINCE40) $(KERNEL_PATCH_SINCE41) $(KERNEL_PATCH_SINCE42) $(KERNEL_PATCH_SINCE43) $(KERNEL_PATCH_SINCE44) $(KERNEL_PATCH_SINCE45) $(KERNEL_PATCH_SINCE46) $(KERNEL_PATCH_SINCE47) $(KERNEL_PATCH_SINCE48) $(KERNEL_PATCH_SINCE49) $(KERNEL_PATCH_SINCE50) $(KERNEL_PATCH_SINCE51) $(KERNEL_PATCH_SINCE52) $(KERNEL_PATCH_SINCE53) $(KERNEL_PATCH_SINCE54)
KERNEL_DEFCONFIG := exynos850-a12snsxx_defconfig
# 4.19 wants clang 9 (AOSP r353983c family). clang-17 on R620 is too new.
CLANG_BIN ?= os/third_party/clang-9/bin
# BOOT partition on SM-A127F is 44 MiB; stock boot.img is padded to this with AVB.
BOOT_PART_SIZE := 46137344
BL_ZIP ?= /srv/media/BL_A127FXXSDDXJ2_QB86601342_REV00_user_low_ship_MULTI_CERT.tar.md5.zip
MEDIA ?= /srv/media

.PHONY: all unpack init ramdisk boot.img vbmeta odin-tar restore flash check-stock busybox dropbear \
	up_param up_param-flash boot-v012 boot-v013 boot-v014 boot-v015 boot-v016 boot-v017 boot-v018 boot-v019 boot-v020 boot-v021 boot-v022 boot-v023 boot-v024 boot-v025 boot-v026 boot-v027 boot-v028 boot-v029 boot-v030 boot-v031 kernel kernel-patch touchlab beep play abox-fw wifi-fw iw wpa

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

$(BUILD)/touchlab: os/init/touchlab.c
	mkdir -p $(BUILD)
	$(CROSS)gcc -static -Os -s -o $@ os/init/touchlab.c

touchlab: $(BUILD)/touchlab

$(BUILD)/beep: os/init/beep.c
	@test -f $(TINYALSA)/include/tinyalsa/asoundlib.h || { echo "missing $(TINYALSA) — git clone --depth 1 https://github.com/tinyalsa/tinyalsa $(TINYALSA)"; exit 1; }
	mkdir -p $(BUILD)
	$(CROSS)gcc -static -Os -s -I$(TINYALSA)/include -I$(TINYALSA)/src -o $@ os/init/beep.c \
		$(TINYALSA)/src/pcm.c $(TINYALSA)/src/mixer.c \
		$(TINYALSA)/src/pcm_hw.c $(TINYALSA)/src/mixer_hw.c \
		$(TINYALSA)/src/limits.c $(TINYALSA)/src/pcm_plugin.c \
		$(TINYALSA)/src/mixer_plugin.c $(TINYALSA)/src/snd_card_plugin.c \
		-ldl

beep: $(BUILD)/beep

$(BUILD)/play: os/init/play.c
	@test -f $(TINYALSA)/include/tinyalsa/asoundlib.h || { echo "missing $(TINYALSA) — git clone --depth 1 https://github.com/tinyalsa/tinyalsa $(TINYALSA)"; exit 1; }
	mkdir -p $(BUILD)
	$(CROSS)gcc -static -Os -s -I$(TINYALSA)/include -I$(TINYALSA)/src -o $@ os/init/play.c \
		$(TINYALSA)/src/pcm.c $(TINYALSA)/src/mixer.c \
		$(TINYALSA)/src/pcm_hw.c $(TINYALSA)/src/mixer_hw.c \
		$(TINYALSA)/src/limits.c $(TINYALSA)/src/pcm_plugin.c \
		$(TINYALSA)/src/mixer_plugin.c $(TINYALSA)/src/snd_card_plugin.c \
		-ldl

play: $(BUILD)/play

$(BUILD)/sounds/test.wav: os/init/gen_test_wav.py
	mkdir -p $(BUILD)/sounds
	python3 os/init/gen_test_wav.py $@

abox-fw: $(VENDOR_IMG)
	@test -x $(DEBUGFS) || { echo "missing $(DEBUGFS) to extract ABOX firmware from vendor.img"; exit 1; }
	mkdir -p $(ABOX_FW)
	@for f in $(ABOX_FW_BINS); do \
		$(DEBUGFS) -R "dump /firmware/$$f $(CURDIR)/$(ABOX_FW)/$$f" $(VENDOR_IMG) >/dev/null; \
		test -s $(ABOX_FW)/$$f || { echo "failed dump $$f"; exit 1; } \
	done
	@echo "abox-fw $$(du -sb $(ABOX_FW) | cut -f1) bytes"

wifi-fw: $(VENDOR_IMG)
	@test -x $(DEBUGFS) || { echo "missing $(DEBUGFS) to extract Maxwell firmware from vendor.img"; exit 1; }
	rm -rf $(WIFI_FW)
	mkdir -p $(WIFI_FW)
	@for f in $(WIFI_FW_FILES); do \
		$(DEBUGFS) -R "dump /etc/wifi/$$f $(CURDIR)/$(WIFI_FW)/$$f" $(VENDOR_IMG) >/dev/null; \
		test -e $(WIFI_FW)/$$f || { echo "failed dump $$f"; exit 1; } \
	done
	@test -s $(WIFI_FW)/mx140.bin || { echo "failed dump mx140.bin"; exit 1; }
	@echo "wifi-fw $$(du -sb $(WIFI_FW) | cut -f1) bytes (mx140.bin $$(wc -c < $(WIFI_FW)/mx140.bin))"

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

# Static aarch64 iw 6.9 + libnl-3.11 (nl80211). Built on R620; gitignored.
# Rebuild: /tmp/e3-build-iw.sh or the recipe below.
$(IW):
	@test -x $(IW) || { echo "missing $(IW) — cross-build static iw+libnl on R620"; exit 1; }

iw: $(IW)

# Static aarch64 wpa_supplicant 2.11 + wpa_cli (nl80211, internal TLS, libnl-3.11).
# Built on R620 for LIVE join; gitignored. Do not pack wpa_passphrase (encourages baked PSKs).
$(WPA_SUPPLICANT) $(WPA_CLI):
	@test -x $(WPA_SUPPLICANT) || { echo "missing $(WPA_SUPPLICANT) — copy R620 /tmp/saaios-wpa/out/wpa_supplicant"; exit 1; }
	@test -x $(WPA_CLI) || { echo "missing $(WPA_CLI) — copy R620 /tmp/saaios-wpa/out/wpa_cli"; exit 1; }

wpa: $(WPA_SUPPLICANT) $(WPA_CLI)

init: $(BUILD)/init

ramdisk: $(BUILD)/init $(BUILD)/touchlab $(BUILD)/beep $(BUILD)/play $(BUILD)/sounds/test.wav $(BUSYBOX) $(DROPBEAR) $(DROPBEAR_SCP) $(SFTP_SERVER) $(IW) $(WPA_SUPPLICANT) $(WPA_CLI) $(DROPBEAR_AUTH_KEYS) abox-fw wifi-fw
	rm -rf $(RAMDISK)
	mkdir -p $(RAMDISK)/dev $(RAMDISK)/proc $(RAMDISK)/sys $(RAMDISK)/tmp $(RAMDISK)/run \
		$(RAMDISK)/bin $(RAMDISK)/sbin $(RAMDISK)/etc/dropbear $(RAMDISK)/root \
		$(RAMDISK)/usr/libexec $(RAMDISK)/usr/share/sounds $(RAMDISK)/usr/share/udhcpc \
		$(RAMDISK)/dev/pts
	cp $(BUILD)/init $(RAMDISK)/init
	chmod 0755 $(RAMDISK)/init
	cp $(BUILD)/touchlab $(RAMDISK)/sbin/touchlab
	chmod 0755 $(RAMDISK)/sbin/touchlab
	cp $(BUILD)/beep $(RAMDISK)/sbin/beep
	chmod 0755 $(RAMDISK)/sbin/beep
	cp $(BUILD)/play $(RAMDISK)/sbin/play
	chmod 0755 $(RAMDISK)/sbin/play
	cp $(BUILD)/sounds/test.wav $(RAMDISK)/usr/share/sounds/test.wav
	chmod 0644 $(RAMDISK)/usr/share/sounds/test.wav
	sed 's/\r$$//' os/init/usb-host > $(RAMDISK)/sbin/usb-host
	sed 's/\r$$//' os/init/usb-device > $(RAMDISK)/sbin/usb-device
	sed 's/\r$$//' os/init/wifi-up > $(RAMDISK)/sbin/wifi-up
	sed 's/\r$$//' os/init/wifi-scan > $(RAMDISK)/sbin/wifi-scan
	sed 's/\r$$//' os/init/wifi-join > $(RAMDISK)/sbin/wifi-join
	sed 's/\r$$//' os/init/udhcpc.script > $(RAMDISK)/usr/share/udhcpc/default.script
	cp $(IW) $(RAMDISK)/sbin/iw
	cp $(WPA_SUPPLICANT) $(RAMDISK)/sbin/wpa_supplicant
	cp $(WPA_CLI) $(RAMDISK)/sbin/wpa_cli
	chmod 0755 $(RAMDISK)/sbin/usb-host $(RAMDISK)/sbin/usb-device \
		$(RAMDISK)/sbin/wifi-up $(RAMDISK)/sbin/wifi-scan $(RAMDISK)/sbin/wifi-join \
		$(RAMDISK)/sbin/iw $(RAMDISK)/sbin/wpa_supplicant $(RAMDISK)/sbin/wpa_cli \
		$(RAMDISK)/usr/share/udhcpc/default.script
	mkdir -p $(RAMDISK)/vendor/firmware $(RAMDISK)/vendor/etc/wifi $(RAMDISK)/system/etc $(RAMDISK)/lib
	ln -s /vendor/firmware $(RAMDISK)/lib/firmware
	ln -sfn /vendor/etc/wifi $(RAMDISK)/etc/wifi
	ln -sfn /vendor/etc/wifi $(RAMDISK)/system/etc/wifi
	@for f in $(ABOX_FW_BINS); do cp $(ABOX_FW)/$$f $(RAMDISK)/vendor/firmware/$$f; done
	@for f in $(WIFI_FW_FILES); do cp $(WIFI_FW)/$$f $(RAMDISK)/vendor/etc/wifi/$$f; done
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
	ln -s busybox $(RAMDISK)/bin/lsusb
	ln -s busybox $(RAMDISK)/bin/mkdir
	ln -s busybox $(RAMDISK)/bin/chmod
	ln -s busybox $(RAMDISK)/bin/kill
	ln -s busybox $(RAMDISK)/bin/head
	ln -s busybox $(RAMDISK)/bin/tail
	ln -s busybox $(RAMDISK)/sbin/telnetd
	ln -s busybox $(RAMDISK)/sbin/udhcpd
	ln -s busybox $(RAMDISK)/sbin/udhcpc
	ln -s busybox $(RAMDISK)/bin/route
	ln -s busybox $(RAMDISK)/bin/pidof
	ln -s busybox $(RAMDISK)/bin/sed
	ln -s busybox $(RAMDISK)/bin/rm
	ln -s busybox $(RAMDISK)/bin/cp
	cp $(DROPBEAR) $(RAMDISK)/sbin/dropbear
	cp $(DROPBEAR_SCP) $(RAMDISK)/bin/scp
	cp $(SFTP_SERVER) $(RAMDISK)/usr/libexec/sftp-server
	chmod 0755 $(RAMDISK)/sbin/dropbear $(RAMDISK)/bin/scp $(RAMDISK)/usr/libexec/sftp-server
	cp $(DROPBEAR_AUTH_KEYS) $(RAMDISK)/etc/dropbear/authorized_keys
	cp os/init/shells $(RAMDISK)/etc/shells
	chmod 0644 $(RAMDISK)/etc/shells
	chmod 0700 $(RAMDISK)/etc/dropbear
	chmod 0600 $(RAMDISK)/etc/dropbear/authorized_keys
	sed 's/\r$$//' os/init/rcS > $(RAMDISK)/etc/rcS
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
	@test -f os/third_party/td4150_oss_dxj6/synaptics_tcm_core.c || { echo "missing OSS td4150"; exit 1; }
	mkdir -p $(KERNEL_DIR)/drivers/input/touchscreen/synaptics/td4150
	cp -f os/third_party/td4150_oss_dxj6/synaptics_tcm_*.c \
		os/third_party/td4150_oss_dxj6/synaptics_tcm_*.h \
		os/third_party/td4150_oss_dxj6/Makefile \
		os/third_party/td4150_oss_dxj6/Kconfig \
		$(KERNEL_DIR)/drivers/input/touchscreen/synaptics/td4150/
	@echo "copied OSS td4150 (since76 skip 0x25 after 0x20 + oneshot 0x30) into gitignored kernel tree"
	@if grep -qlE 'read-floor 256 \+ touchlab|since76 touch lab: sysfs saaios_touch|since76 button menu: sysfs saaios_exp|since75 0x05 ENABLE_REPORT 0x11 after 0x25 then live identify\(false\)/0x20|since74 0x23 GET_DYNAMIC_CONFIG DC_NO_DOZE then 0x05 ENABLE_REPORT 0x11 after 0x25|since73 0x05 ENABLE_REPORT payload 0x11 after 0x25|since72 0x26 SET_TOUCH_REPORT_CONFIG kmemdup of 0x25 before unlock|since71 0x26 SET_TOUCH_REPORT_CONFIG echo of 0x25|since70 one-shot late touch_resume after REINIT|since69 leftover REINIT after skip switch_mode|since68 skip switch_mode already HDL 0x02 after 0x45|since67 oneshot 0x45 HDL_WR_CHUNK_SIZE=0|since66 0x1b STATUS_OK completes 0x45 waiter|since65 leftover 0x1b storm guard|since64 chunked 0x45 wr_chunk=1024 good APP_CODE|since64 early zero 0x45 after cold Identify|since63 APP_CODE memset 0 all 97280|since62 APP_CODE\[0\] 0x55->0x54 magic|since61 FORCE RAM corrupt|since60 delayed 0x45 corrupt APP_CODE|since59 delayed 0x45 after panel|since58 delayed 0x45 after panel callback|since57 probe clean OSS|defer 0x45 until panel enabled' $(KERNEL_DIR)/drivers/input/touchscreen/synaptics/td4150/synaptics_tcm_core.c 2>/dev/null || grep -qlE 'since69 leftover REINIT after skip switch_mode|since68 skip switch_mode already HDL 0x02 after 0x45|since67 oneshot 0x45 HDL_WR_CHUNK_SIZE=0|since66 0x1b STATUS_OK completes 0x45 waiter|since65 leftover 0x1b storm guard|since64 chunked 0x45|since64 early zero 0x45|since63 APP_CODE memset|since62 APP_CODE\[0\]|since61 FORCE RAM corrupt|since60 delayed 0x45|since59 delayed 0x45|since58 delayed 0x45' $(KERNEL_DIR)/drivers/input/touchscreen/synaptics/td4150/synaptics_tcm_core.c 2>/dev/null; then \
		echo "OSS td4150 overlay (since76 skip 0x25 after 0x20 + oneshot 0x30): skipping maze KERNEL_PATCHES"; \
	else for p in $(KERNEL_PATCHES); do \
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
			*romboot-skip-0x30-reinit*) m='SAaiOS_TOUCH_DBG: since45 probe' ;; \
			*romboot-identify-dump*) m='SAaiOS_TOUCH_DBG: since46 probe' ;; \
			*romboot-app-code-id*) m='SAaiOS_TOUCH_DBG: since47 probe' ;; \
			*romboot-0x42-after-02*) m='SAaiOS_TOUCH_DBG: since48 probe' ;; \
			*romboot-identify-before-after*) m='SAaiOS_TOUCH_DBG: since49 probe' ;; \
			*romboot-hdl-1b-trace*) m='SAaiOS_TOUCH_DBG: since50 probe' ;; \
			*romboot-hdl-serialize*) m='SAaiOS_TOUCH_DBG: since51 probe' ;; \
			*romboot-hdl-switch-bootloader*) m='SAaiOS_TOUCH_DBG: since52 switch_mode after 0x1b' ;; \
			*romboot-hdl-0x42-from-02*) m='SAaiOS_TOUCH_DBG: since53 probe' ;; \
			*romboot-hdl-no-post-cmd*) m='SAaiOS_TOUCH_DBG: since54 probe' ;; \
			*) m='' ;; \
		esac; \
		if [ -n "$$m" ] && grep -Rql -- "$$m" $(KERNEL_DIR)/drivers/input/touchscreen/synaptics/td4150/; then \
			echo "$$p already applied"; \
		elif patch -d $(KERNEL_DIR) -p1 --forward --dry-run --silent < $$p; then \
			patch -d $(KERNEL_DIR) -p1 --forward < $$p; \
		else \
			echo "FAILED to apply $$p"; exit 1; \
		fi; \
	done; fi

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

# boot-v019: stock DTB + Image. since76 = skip 0x25 after 0x20 OK + oneshot 0x30 + /sbin/touchlab (queued enable_report/app_config/no_doze).
# Do not overwrite since54–since75 tars. Only since76 may be overwritten. See docs/os/targets/sm-a127f/kernel-touch.md.
ifeq ($(wildcard $(KERNEL_IMAGE)),)
boot-v019:
	@echo "boot-v019: Image missing at $(KERNEL_IMAGE)"
	@echo "Apply $(KERNEL_PATCHES), then build with exynos850-a12snsxx_defconfig."
	@echo "See docs/os/targets/sm-a127f/kernel-touch.md."
	@echo "Not packing. Not copying $(MEDIA)/saaios-boot-v019-$(SUBVERSION_V019).tar."
	@echo "Phone stays on current boot. make flash still refuses."
else
boot-v019: unpack ramdisk vbmeta
	@case "$(SUBVERSION_V019)" in since54|since55|since56|since57|since58|since59|since60|since61|since62|since63|since64|since65|since66|since67|since68|since69|since70|since71|since72|since73|since74|since75) echo "REFUSING to overwrite saaios-boot-v019-$(SUBVERSION_V019).tar"; exit 1;; esac
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--kernel $(KERNEL_IMAGE) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7 syna_corrupt_app=1" \
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

# boot-v020: STOCK kernel + DTB (A127FXXSDDXJ2) + ramdisk console.
# No maze Image. TSP parked. Phase C: battery / backlight / keys on fb0.
# Does not flash. Does not overwrite v011 or v019-sinceN tars.
boot-v020: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V020)
	python3 os/bootimg/bootimg.py unpack $(OUT_V020) -o $(BUILD)/boot-v020-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v020-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v020-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v020
	mkdir -p $(BUILD)/odin-v020
	cp $(OUT_V020) $(BUILD)/odin-v020/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v020/vbmeta.img
	tar -C $(BUILD)/odin-v020 -cf $(BUILD)/saaios-boot-v020.tar boot.img vbmeta.img
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v020.tar $(MEDIA)/saaios-boot-v020.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v020.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v020.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v020.tar"; \
	fi
	sha256sum $(OUT_V020)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v020.tar"
	tar -tvf $(BUILD)/saaios-boot-v020.tar

# boot-v021: stock kernel+DTB + ramdisk console + ABOX fw + /sbin/beep.
# Phase E1 speaker beep. Does not overwrite v020. No maze Image.
boot-v021: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V021)
	python3 os/bootimg/bootimg.py unpack $(OUT_V021) -o $(BUILD)/boot-v021-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v021-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v021-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v021
	mkdir -p $(BUILD)/odin-v021
	cp $(OUT_V021) $(BUILD)/odin-v021/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v021/vbmeta.img
	tar -C $(BUILD)/odin-v021 -cf $(BUILD)/saaios-boot-v021.tar boot.img vbmeta.img
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v021.tar $(MEDIA)/saaios-boot-v021.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v021.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v021.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v021.tar"; \
	fi
	sha256sum $(OUT_V021)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v021.tar"
	tar -tvf $(BUILD)/saaios-boot-v021.tar

# boot-v022: v021 + audible SMA1303 (Mode Mono, unmute, ASRC3 off, louder).
# Also dropbear -R so :22 starts. Does not overwrite v021. No maze Image.
boot-v022: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V022)
	python3 os/bootimg/bootimg.py unpack $(OUT_V022) -o $(BUILD)/boot-v022-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v022-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v022-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v022
	mkdir -p $(BUILD)/odin-v022
	cp $(OUT_V022) $(BUILD)/odin-v022/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v022/vbmeta.img
	tar -C $(BUILD)/odin-v022 -cf $(BUILD)/saaios-boot-v022.tar boot.img vbmeta.img
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v022.tar $(MEDIA)/saaios-boot-v022.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v022.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v022.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v022.tar"; \
	fi
	sha256sum $(OUT_V022)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v022.tar"
	tar -tvf $(BUILD)/saaios-boot-v022.tar

# boot-v023: v022 + ABOX Sound Type SPEAKER (live default was VOICE).
# Jack is AUD3004X evdev only — does not mute SMA1303. Does not overwrite v022.
boot-v023: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V023)
	python3 os/bootimg/bootimg.py unpack $(OUT_V023) -o $(BUILD)/boot-v023-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v023-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v023-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v023
	mkdir -p $(BUILD)/odin-v023
	cp $(OUT_V023) $(BUILD)/odin-v023/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v023/vbmeta.img
	tar -C $(BUILD)/odin-v023 -cf $(BUILD)/saaios-boot-v023.tar boot.img vbmeta.img
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v023.tar $(MEDIA)/saaios-boot-v023.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v023.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v023.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v023.tar"; \
	fi
	sha256sum $(OUT_V023)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v023.tar"
	tar -tvf $(BUILD)/saaios-boot-v023.tar

# boot-v024: v023 SPEAKER beep + rcS LF + listeners before USB net + UDC retry.
# Does not overwrite v021/v022/v023. Stock kernel+DTB. No maze Image.
boot-v024: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V024)
	python3 os/bootimg/bootimg.py unpack $(OUT_V024) -o $(BUILD)/boot-v024-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v024-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v024-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v024
	mkdir -p $(BUILD)/odin-v024
	cp $(OUT_V024) $(BUILD)/odin-v024/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v024/vbmeta.img
	tar -C $(BUILD)/odin-v024 -cf $(BUILD)/saaios-boot-v024.tar boot.img vbmeta.img
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v024.tar $(MEDIA)/saaios-boot-v024.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v024.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v024.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v024.tar"; \
	fi
	sha256sum $(OUT_V024)
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v024.tar"
	tar -tvf $(BUILD)/saaios-boot-v024.tar

# boot-v025: v024 USB (LF rcS) + SPEAKER + RDMA3_A TONEGEN_1KHZ.
# Does not overwrite v021/v022/v023/v024. Stock kernel+DTB. No maze Image.
boot-v025: unpack ramdisk vbmeta
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V025)
	python3 os/bootimg/bootimg.py unpack $(OUT_V025) -o $(BUILD)/boot-v025-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v025-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v025-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v025
	mkdir -p $(BUILD)/odin-v025
	cp $(OUT_V025) $(BUILD)/odin-v025/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v025/vbmeta.img
	tar -C $(BUILD)/odin-v025 -cf $(BUILD)/saaios-boot-v025.tar boot.img vbmeta.img
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v025.tar $(MEDIA)/saaios-boot-v025.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v025.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v025.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v025.tar"; \
	fi
	sha256sum $(OUT_V025) $(MEDIA)/saaios-boot-v025.tar $(MEDIA)/saaios-boot-v024.tar
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v025.tar"
	tar -tvf $(BUILD)/saaios-boot-v025.tar

# boot-v026: v025 USB + pcmC0D1p / UAIF1=SIFS1 (S10 GSI; v025 TONEGEN+SIFS0 silent).
# Does not overwrite v021–v025. Stock kernel+DTB. No maze Image.
boot-v026: unpack ramdisk vbmeta
	@test ! -f $(MEDIA)/saaios-boot-v021.tar || test -s $(MEDIA)/saaios-boot-v021.tar
	@test ! -f $(MEDIA)/saaios-boot-v025.tar || test -s $(MEDIA)/saaios-boot-v025.tar
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V026)
	python3 os/bootimg/bootimg.py unpack $(OUT_V026) -o $(BUILD)/boot-v026-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v026-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v026-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v026
	mkdir -p $(BUILD)/odin-v026
	cp $(OUT_V026) $(BUILD)/odin-v026/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v026/vbmeta.img
	tar -C $(BUILD)/odin-v026 -cf $(BUILD)/saaios-boot-v026.tar boot.img vbmeta.img
	@if [ -e $(MEDIA)/saaios-boot-v025.tar ]; then \
		cmp -s $(BUILD)/saaios-boot-v026.tar $(MEDIA)/saaios-boot-v025.tar && { echo "REFUSING: v026 tar matches v025"; exit 1; } || true; \
	fi
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v026.tar $(MEDIA)/saaios-boot-v026.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v026.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v026.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v026.tar"; \
	fi
	sha256sum $(OUT_V026) $(MEDIA)/saaios-boot-v026.tar $(MEDIA)/saaios-boot-v025.tar
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v026.tar"
	tar -tvf $(BUILD)/saaios-boot-v026.tar

# boot-v027: v026 D1/SIFS1 route + /sbin/play WAV (no auto-play). Power tap stays beep.
# Does not overwrite v021–v026. Stock kernel+DTB. No maze Image.
boot-v027: unpack ramdisk vbmeta
	@test ! -f $(MEDIA)/saaios-boot-v021.tar || test -s $(MEDIA)/saaios-boot-v021.tar
	@test ! -f $(MEDIA)/saaios-boot-v026.tar || test -s $(MEDIA)/saaios-boot-v026.tar
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V027)
	python3 os/bootimg/bootimg.py unpack $(OUT_V027) -o $(BUILD)/boot-v027-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v027-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v027-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v027
	mkdir -p $(BUILD)/odin-v027
	cp $(OUT_V027) $(BUILD)/odin-v027/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v027/vbmeta.img
	tar -C $(BUILD)/odin-v027 -cf $(BUILD)/saaios-boot-v027.tar boot.img vbmeta.img
	@if [ -e $(MEDIA)/saaios-boot-v026.tar ]; then \
		cmp -s $(BUILD)/saaios-boot-v027.tar $(MEDIA)/saaios-boot-v026.tar && { echo "REFUSING: v027 tar matches v026"; exit 1; } || true; \
	fi
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v027.tar $(MEDIA)/saaios-boot-v027.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v027.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v027.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v027.tar"; \
	fi
	sha256sum $(OUT_V027) $(MEDIA)/saaios-boot-v027.tar $(MEDIA)/saaios-boot-v026.tar
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v027.tar"
	tar -tvf $(BUILD)/saaios-boot-v027.tar

# boot-v028: v027 audio + explicit /sbin/usb-host and /sbin/usb-device.
# Boot stays RNDIS device. Does not overwrite v021–v027. Stock kernel+DTB.
boot-v028: unpack ramdisk vbmeta
	@test ! -f $(MEDIA)/saaios-boot-v027.tar || test -s $(MEDIA)/saaios-boot-v027.tar
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V028)
	python3 os/bootimg/bootimg.py unpack $(OUT_V028) -o $(BUILD)/boot-v028-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v028-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v028-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v028
	mkdir -p $(BUILD)/odin-v028
	cp $(OUT_V028) $(BUILD)/odin-v028/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v028/vbmeta.img
	tar -C $(BUILD)/odin-v028 -cf $(BUILD)/saaios-boot-v028.tar boot.img vbmeta.img
	@if [ -e $(MEDIA)/saaios-boot-v027.tar ]; then \
		cmp -s $(BUILD)/saaios-boot-v028.tar $(MEDIA)/saaios-boot-v027.tar && { echo "REFUSING: v028 tar matches v027"; exit 1; } || true; \
	fi
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v028.tar $(MEDIA)/saaios-boot-v028.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v028.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v028.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v028.tar"; \
	fi
	sha256sum $(OUT_V028) $(MEDIA)/saaios-boot-v028.tar $(MEDIA)/saaios-boot-v027.tar
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v028.tar"
	tar -tvf $(BUILD)/saaios-boot-v028.tar

# boot-v029: v028 + Maxwell mx140 firmware + /sbin/wifi-up (not at boot).
# Does not overwrite v021–v028. Stock kernel+DTB. No maze Image.
boot-v029: unpack ramdisk vbmeta
	@test ! -f $(MEDIA)/saaios-boot-v028.tar || test -s $(MEDIA)/saaios-boot-v028.tar
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V029)
	python3 os/bootimg/bootimg.py unpack $(OUT_V029) -o $(BUILD)/boot-v029-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v029-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v029-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v029
	mkdir -p $(BUILD)/odin-v029
	cp $(OUT_V029) $(BUILD)/odin-v029/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v029/vbmeta.img
	tar -C $(BUILD)/odin-v029 -cf $(BUILD)/saaios-boot-v029.tar boot.img vbmeta.img
	@if [ -e $(MEDIA)/saaios-boot-v028.tar ]; then \
		cmp -s $(BUILD)/saaios-boot-v029.tar $(MEDIA)/saaios-boot-v028.tar && { echo "REFUSING: v029 tar matches v028"; exit 1; } || true; \
	fi
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v029.tar $(MEDIA)/saaios-boot-v029.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v029.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v029.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v029.tar"; \
	fi
	sha256sum $(OUT_V029) $(MEDIA)/saaios-boot-v029.tar $(MEDIA)/saaios-boot-v028.tar
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v029.tar"
	tar -tvf $(BUILD)/saaios-boot-v029.tar

# boot-v030: v029 + static /sbin/iw + /sbin/wifi-scan (telnet-only, not at boot).
# Does not overwrite v021–v029. Stock kernel+DTB. No maze Image. No wpa_supplicant.
boot-v030: unpack ramdisk vbmeta
	@test ! -f $(MEDIA)/saaios-boot-v029.tar || test -s $(MEDIA)/saaios-boot-v029.tar
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V030)
	python3 os/bootimg/bootimg.py unpack $(OUT_V030) -o $(BUILD)/boot-v030-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v030-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v030-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v030
	mkdir -p $(BUILD)/odin-v030
	cp $(OUT_V030) $(BUILD)/odin-v030/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v030/vbmeta.img
	tar -C $(BUILD)/odin-v030 -cf $(BUILD)/saaios-boot-v030.tar boot.img vbmeta.img
	@if [ -e $(MEDIA)/saaios-boot-v029.tar ]; then \
		cmp -s $(BUILD)/saaios-boot-v030.tar $(MEDIA)/saaios-boot-v029.tar && { echo "REFUSING: v030 tar matches v029"; exit 1; } || true; \
	fi
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v030.tar $(MEDIA)/saaios-boot-v030.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v030.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v030.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v030.tar"; \
	fi
	sha256sum $(OUT_V030) $(MEDIA)/saaios-boot-v030.tar $(MEDIA)/saaios-boot-v029.tar
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v030.tar"
	tar -tvf $(BUILD)/saaios-boot-v030.tar

# boot-v031: v030 + static /sbin/wpa_supplicant + /sbin/wpa_cli + /sbin/wifi-join
# (telnet-only, not at boot). Conf/PSK never packed. Stock kernel+DTB. No maze Image.
# Does not overwrite v021–v030.
boot-v031: unpack ramdisk vbmeta
	@test ! -f $(MEDIA)/saaios-boot-v030.tar || test -s $(MEDIA)/saaios-boot-v030.tar
	@test -f $(STOCK_UNPACK)/dtb || { echo "missing stock dtb at $(STOCK_UNPACK)/dtb"; exit 1; }
	python3 os/bootimg/bootimg.py pack $(STOCK_UNPACK) \
		--ramdisk $(BUILD)/ramdisk.cpio.gz \
		--cmdline "androidboot.hardware=exynos850 androidboot.selinux=permissive loop.max_part=7" \
		--seandroid --pad-to $(BOOT_PART_SIZE) \
		-o $(OUT_V031)
	python3 os/bootimg/bootimg.py unpack $(OUT_V031) -o $(BUILD)/boot-v031-check
	cmp $(STOCK_UNPACK)/dtb $(BUILD)/boot-v031-check/dtb
	cmp $(STOCK_UNPACK)/kernel $(BUILD)/boot-v031-check/kernel
	@echo "packed stock DXJ2 kernel+DTB (not Project-Xed Image)"
	rm -rf $(BUILD)/odin-v031
	mkdir -p $(BUILD)/odin-v031
	cp $(OUT_V031) $(BUILD)/odin-v031/boot.img
	cp $(VBMETA_OUT) $(BUILD)/odin-v031/vbmeta.img
	tar -C $(BUILD)/odin-v031 -cf $(BUILD)/saaios-boot-v031.tar boot.img vbmeta.img
	@if [ -e $(MEDIA)/saaios-boot-v030.tar ]; then \
		cmp -s $(BUILD)/saaios-boot-v031.tar $(MEDIA)/saaios-boot-v030.tar && { echo "REFUSING: v031 tar matches v030"; exit 1; } || true; \
	fi
	@if [ -d $(MEDIA) ]; then \
		cp $(BUILD)/saaios-boot-v031.tar $(MEDIA)/saaios-boot-v031.tar; \
		chmod 664 $(MEDIA)/saaios-boot-v031.tar; \
		chown mike:media $(MEDIA)/saaios-boot-v031.tar 2>/dev/null || true; \
		echo "copied $(MEDIA)/saaios-boot-v031.tar"; \
	fi
	sha256sum $(OUT_V031) $(MEDIA)/saaios-boot-v031.tar $(MEDIA)/saaios-boot-v030.tar
	@echo "Odin AP tar (not flashed):"
	@echo "  $(BUILD)/saaios-boot-v031.tar"
	tar -tvf $(BUILD)/saaios-boot-v031.tar

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
	@echo "  v019: $(OUT_V019) (only if $(KERNEL_IMAGE) was packed)"
	@echo "  v020: $(OUT_V020) (stock kernel + ramdisk console; TSP parked)"
	@echo "  v021: $(OUT_V021) (stock kernel + ABOX fw + /sbin/beep; Power tap speaker)"
	@echo "  v022: $(OUT_V022) (v021 + SMA Mode Mono/unmute; dropbear -R)"
	@echo "  v023: $(OUT_V023) (v022 + ABOX Sound Type SPEAKER; jack log)"
	@echo "  v024: $(OUT_V024) (v023 + rcS LF/listeners first; UDC retry)"
	@echo "  v025: $(OUT_V025) (v024 USB + RDMA3_A TONEGEN_1KHZ)"
	@echo "  v026: $(OUT_V026) (v025 USB + pcmC0D1p UAIF1=SIFS1)"
	@echo "  v027: $(OUT_V027) (v026 route + /sbin/play WAV)"
	@echo "  v028: $(OUT_V028) (v027 + explicit usb-host/usb-device; boot stays device)"
	@echo "  v029: $(OUT_V029) (v028 + Maxwell mx140 fw + /sbin/wifi-up; not at boot)"
	@echo "  v030: $(OUT_V030) (v029 + static /sbin/iw + /sbin/wifi-scan; not at boot)"
	@echo "  v031: $(OUT_V031) (v030 + wpa_supplicant + wifi-join by args; not at boot)"
	@echo "Rollback: make restore images in $(STOCK)"
	@echo "PARAM nologo is a separate Odin BL tar; make up_param-flash also refuses."
	@exit 2
