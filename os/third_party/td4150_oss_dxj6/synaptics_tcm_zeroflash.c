/*
 * Synaptics TCM touchscreen driver
 *
 * Copyright (C) 2017-2018 Synaptics Incorporated. All rights reserved.
 *
 * Copyright (C) 2017-2018 Scott Lin <scott.lin@tw.synaptics.com>
 * Copyright (C) 2018-2019 Ian Su <ian.su@tw.synaptics.com>
 * Copyright (C) 2018-2019 Joey Zhou <joey.zhou@synaptics.com>
 * Copyright (C) 2018-2019 Yuehao Qiu <yuehao.qiu@synaptics.com>
 * Copyright (C) 2018-2019 Aaron Chen <aaron.chen@tw.synaptics.com>
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * INFORMATION CONTAINED IN THIS DOCUMENT IS PROVIDED "AS-IS," AND SYNAPTICS
 * EXPRESSLY DISCLAIMS ALL EXPRESS AND IMPLIED WARRANTIES, INCLUDING ANY
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE,
 * AND ANY WARRANTIES OF NON-INFRINGEMENT OF ANY INTELLECTUAL PROPERTY RIGHTS.
 * IN NO EVENT SHALL SYNAPTICS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, PUNITIVE, OR CONSEQUENTIAL DAMAGES ARISING OUT OF OR IN CONNECTION
 * WITH THE USE OF THE INFORMATION CONTAINED IN THIS DOCUMENT, HOWEVER CAUSED
 * AND BASED ON ANY THEORY OF LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * NEGLIGENCE OR OTHER TORTIOUS ACTION, AND EVEN IF SYNAPTICS WAS ADVISED OF
 * THE POSSIBILITY OF SUCH DAMAGE. IF A TRIBUNAL OF COMPETENT JURISDICTION DOES
 * NOT PERMIT THE DISCLAIMER OF DIRECT DAMAGES OR ANY OTHER DAMAGES, SYNAPTICS'
 * TOTAL CUMULATIVE LIABILITY TO ANY PARTY SHALL NOT EXCEED ONE HUNDRED U.S.
 * DOLLARS.
 */

#include <linux/gpio.h>
#include <linux/crc32.h>
#include <linux/firmware.h>
#include <linux/ktime.h>
#include <linux/atomic.h>
#include <linux/moduleparam.h>
#include <linux/slab.h>
#include <linux/string.h>
#include <crypto/hash.h>
#include <crypto/sha.h>
#include "synaptics_tcm_core.h"

static int syna_corrupt_app;
module_param(syna_corrupt_app, int, 0644);
MODULE_PARM_DESC(syna_corrupt_app,
	"If 1, flip one RAM byte of APP_CODE immediately before 0x45 (default 0). Does not modify the firmware file.");

static int __init syna_corrupt_app_setup(char *str)
{
	get_option(&str, &syna_corrupt_app);
	return 1;
}
__setup("syna_corrupt_app=", syna_corrupt_app_setup);

/* since60: boot.img had syna_corrupt_app=1; Samsung BL dropped it
 * (/proc/cmdline had no token). Do not rely on cmdline / __setup.
 */
#ifndef SAAIOS_FORCE_CORRUPT_APP
#define SAAIOS_FORCE_CORRUPT_APP	0
#endif

extern char *saved_command_line;

#define SAAIOS_APP_CODE_SHA256_GOOD \
	"034f1e842d1a01f318ec0cda18c26ad5a6a72946cba50e30c485066222374165"
#define SAAIOS_APP_CODE_CRC32_GOOD	0x5a555e91U
/* since63: memset entire APP_CODE still TD4150-12.0.12 under oneshot 97k.
 * since64: unmodified APP_CODE + stock write_message chunking (wr_chunk=1024).
 */
#define SAAIOS_APP_CODE_WIPE_SIZE	97280U

static void syna_sha256_hex(const unsigned char *data, unsigned int len,
		char *out, size_t outlen)
{
	struct crypto_shash *tfm;
	struct shash_desc *desc;
	u8 digest[SHA256_DIGEST_SIZE];
	unsigned int i;

	if (!out || outlen < 65)
		return;
	memset(out, 0, outlen);
	if (!data) {
		scnprintf(out, outlen, "null");
		return;
	}
	/* SHASH_DESC_ON_STACK(desc, tfm) MUST run after tfm is allocated.
	 * since57 called it with an uninitialized tfm: descsize was garbage,
	 * the VLA overflowed into digest[], and dmesg printed 16 zero bytes
	 * then the real last 16 bytes of known_good. Print bug, not a bad
	 * APP_CODE (APP_CODE[0..3] is 55 aa 01 00; reserved[0] is size>>16).
	 */
	tfm = crypto_alloc_shash("sha256", 0, 0);
	if (IS_ERR(tfm)) {
		scnprintf(out, outlen, "alloc-fail");
		return;
	}
	{
		SHASH_DESC_ON_STACK(desc_stk, tfm);
		desc = desc_stk;
		desc->tfm = tfm;
		desc->flags = 0;
		if (crypto_shash_digest(desc, data, len, digest)) {
			scnprintf(out, outlen, "digest-fail");
			shash_desc_zero(desc);
			crypto_free_shash(tfm);
			return;
		}
		for (i = 0; i < SHA256_DIGEST_SIZE; i++)
			sprintf(out + i * 2, "%02x", digest[i]);
		out[64] = '\0';
		shash_desc_zero(desc);
	}
	crypto_free_shash(tfm);
}

#ifdef CONFIG_SAMSUNG_PRODUCT_SHIP
#define ENABLE_SYS_ZEROFLASH false
#else
#define ENABLE_SYS_ZEROFLASH true
#endif

#define FW_IMAGE_NAME "synaptics/hdl_firmware.img"

#define BOOT_CONFIG_ID "BOOT_CONFIG"

#define F35_APP_CODE_ID "F35_APP_CODE"

#define ROMBOOT_APP_CODE_ID "ROMBOOT_APP_CODE"

#define RESERVED_BYTES 14

#define APP_CONFIG_ID "APP_CONFIG"

#define DISP_CONFIG_ID "DISPLAY"

#define OPEN_SHORT_ID "OPENSHORT"

#define SYSFS_DIR_NAME "zeroflash"

#define IMAGE_FILE_MAGIC_VALUE 0x4818472b

#define FLASH_AREA_MAGIC_VALUE 0x7c05e516

#define PDT_START_ADDR 0x00e9

#define PDT_END_ADDR 0x00ee

#define UBL_FN_NUMBER 0x35

#define F35_CTRL3_OFFSET 18

#define F35_CTRL7_OFFSET 22

#define F35_WRITE_FW_TO_PMEM_COMMAND 4

#define TP_RESET_TO_HDL_DELAY_MS 0

#define DOWNLOAD_RETRY_COUNT 10

enum f35_error_code {
	SUCCESS = 0,
	UNKNOWN_FLASH_PRESENT,
	MAGIC_NUMBER_NOT_PRESENT,
	INVALID_BLOCK_NUMBER,
	BLOCK_NOT_ERASED,
	NO_FLASH_PRESENT,
	CHECKSUM_FAILURE,
	WRITE_FAILURE,
	INVALID_COMMAND,
	IN_DEBUG_MODE,
	INVALID_HEADER,
	REQUESTING_FIRMWARE,
	INVALID_CONFIGURATION,
	DISABLE_BLOCK_PROTECT_FAILURE,
};

enum config_download {
	HDL_INVALID = 0,
	HDL_TOUCH_CONFIG,
	HDL_DISPLAY_CONFIG,
	HDL_OPEN_SHORT_CONFIG,
};

struct area_descriptor {
	unsigned char magic_value[4];
	unsigned char id_string[16];
	unsigned char flags[4];
	unsigned char flash_addr_words[4];
	unsigned char length[4];
	unsigned char checksum[4];
};

struct block_data {
	const unsigned char *data;
	unsigned int size;
	unsigned int flash_addr;
};

struct image_info {
	unsigned int packrat_number;
	struct block_data boot_config;
	struct block_data app_firmware;
	struct block_data app_config;
	struct block_data disp_config;
	struct block_data open_short_config;
};

struct image_header {
	unsigned char magic_value[4];
	unsigned char num_of_areas[4];
};

struct rmi_f35_query {
	unsigned char version:4;
	unsigned char has_debug_mode:1;
	unsigned char has_data5:1;
	unsigned char has_query1:1;
	unsigned char has_query2:1;
	unsigned char chunk_size;
	unsigned char has_ctrl7:1;
	unsigned char has_host_download:1;
	unsigned char has_spi_master:1;
	unsigned char advanced_recovery_mode:1;
	unsigned char reserved:4;
} __packed;

struct rmi_f35_data {
	unsigned char error_code:5;
	unsigned char recovery_mode_forced:1;
	unsigned char nvm_programmed:1;
	unsigned char in_recovery:1;
} __packed;

struct rmi_pdt_entry {
	unsigned char query_base_addr;
	unsigned char command_base_addr;
	unsigned char control_base_addr;
	unsigned char data_base_addr;
	unsigned char intr_src_count:3;
	unsigned char reserved_1:2;
	unsigned char fn_version:2;
	unsigned char reserved_2:1;
	unsigned char fn_number;
} __packed;

struct rmi_addr {
	unsigned short query_base;
	unsigned short command_base;
	unsigned short control_base;
	unsigned short data_base;
};

struct firmware_status {
	unsigned short invalid_static_config:1;
	unsigned short need_disp_config:1;
	unsigned short need_app_config:1;
	unsigned short hdl_version:4;
	unsigned short need_open_short_config:1;
	unsigned short reserved:8;
} __packed;

struct zeroflash_hcd {
	bool has_hdl;
	bool f35_ready;
	bool has_open_short_config;
	const unsigned char *image;
	unsigned char *buf;
	const struct firmware *fw_entry;
	struct work_struct config_work;
	struct delayed_work romboot_delay_work;
	struct workqueue_struct *workqueue;
	struct kobject *sysfs_dir;
	struct rmi_addr f35_addr;
	struct image_info image_info;
	struct firmware_status fw_status;
	struct syna_tcm_buffer out;
	struct syna_tcm_buffer resp;
	struct syna_tcm_hcd *tcm_hcd;
	ktime_t panel_exit_ktime;
	u64 panel_exit_boot_ns;
	unsigned int panel_cb_n;
};

static void zeroflash_do_romboot_firmware_download(void);
static void zeroflash_download_config_work(struct work_struct *work);

DECLARE_COMPLETION(zeroflash_remove_complete);

STORE_PROTOTYPE(zeroflash, hdl)

static struct device_attribute *attrs[] = {
	ATTRIFY(hdl),
};

static struct zeroflash_hcd *zeroflash_hcd;
static atomic_t hdl_status_seen = ATOMIC_INIT(0);

static int zeroflash_wait_hdl(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;

	msleep(HOST_DOWNLOAD_WAIT_MS);

	if (!atomic_read(&tcm_hcd->host_downloading))
		return 0;

	retval = wait_event_interruptible_timeout(tcm_hcd->hdl_wq,
			!atomic_read(&tcm_hcd->host_downloading),
			msecs_to_jiffies(HOST_DOWNLOAD_TIMEOUT_MS));
	if (retval == 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Timed out waiting for completion of host download\n");
		atomic_set(&tcm_hcd->host_downloading, 0);
		retval = -EIO;
	} else {
		retval = 0;
	}

	return retval;
}

static ssize_t zeroflash_sysfs_hdl_store(struct device *dev,
		struct device_attribute *attr, const char *buf, size_t count)
{
	int retval = 0;
	unsigned int input;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;

	if (sscanf(buf, "%u", &input) != 1)
		return -EINVAL;

	if (input && (tcm_hcd->in_hdl_mode)) {

		retval = tcm_hcd->reset(tcm_hcd);
		if (retval < 0)
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to trigger the host download by reset\n");

		retval = zeroflash_wait_hdl(tcm_hcd);
		if (retval < 0)
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to wait for completion of host download\n");

		if (zeroflash_hcd->fw_entry) {
			release_firmware(zeroflash_hcd->fw_entry);
			zeroflash_hcd->fw_entry = NULL;
		}

		zeroflash_hcd->image = NULL;

	} else {
		input_err(true, tcm_hcd->pdev->dev.parent, "Invalid HDL devices\n");
	}
	return count;
}

static int zeroflash_check_uboot(void)
{
	int retval;
	unsigned char fn_number;
	unsigned int retry = 3;
	struct rmi_f35_query query;
	struct rmi_pdt_entry p_entry;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;

re_check:
	retval = syna_tcm_rmi_read(tcm_hcd,
			PDT_END_ADDR,
			&fn_number,
			sizeof(fn_number));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to read RMI function number\n");
		return retval;
	}

	input_dbg(true, tcm_hcd->pdev->dev.parent, "Found F$%02x\n", fn_number);

	if (fn_number != UBL_FN_NUMBER) {
		if (retry--)
			goto re_check;

		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to find F$35\n");
		return -ENODEV;
	}

	if (zeroflash_hcd->f35_ready)
		return 0;

	retval = syna_tcm_rmi_read(tcm_hcd, PDT_START_ADDR,
				(unsigned char *)&p_entry, sizeof(p_entry));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to read PDT entry\n");
		return retval;
	}

	zeroflash_hcd->f35_addr.query_base = p_entry.query_base_addr;
	zeroflash_hcd->f35_addr.command_base = p_entry.command_base_addr;
	zeroflash_hcd->f35_addr.control_base = p_entry.control_base_addr;
	zeroflash_hcd->f35_addr.data_base = p_entry.data_base_addr;

	retval = syna_tcm_rmi_read(tcm_hcd, zeroflash_hcd->f35_addr.query_base,
			(unsigned char *)&query, sizeof(query));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to read F$35 query\n");
		return retval;
	}

	zeroflash_hcd->f35_ready = true;

	if (query.has_query2 && query.has_ctrl7 && query.has_host_download) {
		zeroflash_hcd->has_hdl = true;
	} else {
		input_err(true, tcm_hcd->pdev->dev.parent, "Host download not supported\n");
		zeroflash_hcd->has_hdl = false;
		return -ENODEV;
	}

	return 0;
}

static int zeroflash_parse_fw_image(void)
{
	unsigned int idx;
	unsigned int addr;
	unsigned int offset;
	unsigned int length;
	unsigned int checksum;
	unsigned int flash_addr;
	unsigned int magic_value;
	unsigned int num_of_areas;
	struct image_header *header;
	struct image_info *image_info;
	struct area_descriptor *descriptor;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;
	const unsigned char *image;
	const unsigned char *content;

	if (tcm_hcd->get_fw == 1) {
		image = tcm_hcd->image;
		if (!image)
			image = zeroflash_hcd->image;
	} else {
		image = zeroflash_hcd->image;
	}

	if (tcm_hcd->force_update)
		tcm_hcd->sensor_type = TYPE_ROMBOOT;

	image_info = &zeroflash_hcd->image_info;
	header = (struct image_header *)image;

	magic_value = le4_to_uint(header->magic_value);
	if (magic_value != IMAGE_FILE_MAGIC_VALUE) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Invalid image file magic value\n");
		return -EINVAL;
	}

	memset(image_info, 0x00, sizeof(*image_info));

	offset = sizeof(*header);
	num_of_areas = le4_to_uint(header->num_of_areas);

	for (idx = 0; idx < num_of_areas; idx++) {
		addr = le4_to_uint(image + offset);
		descriptor = (struct area_descriptor *)(image + addr);
		offset += 4;

		magic_value = le4_to_uint(descriptor->magic_value);
		if (magic_value != FLASH_AREA_MAGIC_VALUE)
			continue;

		length = le4_to_uint(descriptor->length);
		content = (unsigned char *)descriptor + sizeof(*descriptor);
		flash_addr = le4_to_uint(descriptor->flash_addr_words) * 2;
		checksum = le4_to_uint(descriptor->checksum);

		if (0 == strncmp((char *)descriptor->id_string, 
			BOOT_CONFIG_ID, strlen(BOOT_CONFIG_ID))) {

			if (checksum != (crc32(~0, content, length) ^ ~0)) {
				input_err(true, tcm_hcd->pdev->dev.parent,
						"Boot config checksum error\n");
				return -EINVAL;
			}
			image_info->boot_config.size = length;
			image_info->boot_config.data = content;
			image_info->boot_config.flash_addr = flash_addr;
			input_info(true, tcm_hcd->pdev->dev.parent,
					"Boot config size = %d\n", length);
			input_info(true, tcm_hcd->pdev->dev.parent,
					"Boot config flash address = 0x%08x\n", flash_addr);
		} else if ((0 == strncmp((char *)descriptor->id_string,
				F35_APP_CODE_ID, strlen(F35_APP_CODE_ID)))) {

			if (tcm_hcd->sensor_type != TYPE_F35) {
				input_err(true, tcm_hcd->pdev->dev.parent,
						"Improper descriptor, F35_APP_CODE_ID\n");
				return -EINVAL;
			}

			if (checksum != (crc32(~0, content, length) ^ ~0)) {
				input_err(true, tcm_hcd->pdev->dev.parent,
						"HDL_F35 firmware checksum error\n");
				return -EINVAL;
			}
			image_info->app_firmware.size = length;
			image_info->app_firmware.data = content;
			image_info->app_firmware.flash_addr = flash_addr;
			input_info(true, tcm_hcd->pdev->dev.parent,
					"HDL_F35 firmware size = %d\n", length);
			input_info(true, tcm_hcd->pdev->dev.parent,
					"HDL_F35 firmware flash address = 0x%08x\n", flash_addr);

		} else if ((0 == strncmp((char *)descriptor->id_string,
				ROMBOOT_APP_CODE_ID,
				strlen(ROMBOOT_APP_CODE_ID)))) {

			if (tcm_hcd->sensor_type != TYPE_ROMBOOT) {
				input_err(true, tcm_hcd->pdev->dev.parent,
						"Improper descriptor, ROMBOOT_APP_CODE_ID\n");
				return -EINVAL;
			}

			if (checksum != (crc32(~0, content, length) ^ ~0)) {
				input_err(true, tcm_hcd->pdev->dev.parent,
						"HDL_ROMBoot firmware checksum error\n");
				return -EINVAL;
			}
			image_info->app_firmware.size = length;
			image_info->app_firmware.data = content;
			image_info->app_firmware.flash_addr = flash_addr;
			input_info(true, tcm_hcd->pdev->dev.parent,
					"HDL_ROMBoot firmware size = %d\n", length);
			input_info(true, tcm_hcd->pdev->dev.parent,
					"HDL_ROMBoot firmware flash address = 0x%08x\n", flash_addr);

		} else if (0 == strncmp((char *)descriptor->id_string,
				APP_CONFIG_ID, strlen(APP_CONFIG_ID))) {

			if (checksum != (crc32(~0, content, length) ^ ~0)) {
				input_err(true, tcm_hcd->pdev->dev.parent,
						"Application config checksum error\n");
				return -EINVAL;
			}
			image_info->app_config.size = length;
			image_info->app_config.data = content;
			image_info->app_config.flash_addr = flash_addr;
			image_info->packrat_number = le4_to_uint(&content[14]);

			for (idx = 0; idx < 4; idx++)
				tcm_hcd->img_version[idx] = content[18 + idx];

			input_info(true, tcm_hcd->pdev->dev.parent,
					"Application config size = %d\n", length);
			input_info(true, tcm_hcd->pdev->dev.parent,
					"Application config flash address = 0x%08x\n", flash_addr);
		} else if (0 == strncmp((char *)descriptor->id_string,
				DISP_CONFIG_ID, strlen(DISP_CONFIG_ID))) {

			if (checksum != (crc32(~0, content, length) ^ ~0)) {
				input_err(true, tcm_hcd->pdev->dev.parent,
						"Display config checksum error\n");
				return -EINVAL;
			}
			image_info->disp_config.size = length;
			image_info->disp_config.data = content;
			image_info->disp_config.flash_addr = flash_addr;
			input_info(true, tcm_hcd->pdev->dev.parent,
					"Display config size = %d\n", length);
			input_info(true, tcm_hcd->pdev->dev.parent,
					"Display config flash address = 0x%08x\n", flash_addr);
		} else if (0 == strncmp((char *)descriptor->id_string,
				OPEN_SHORT_ID, strlen(OPEN_SHORT_ID))) {

			if (checksum != (crc32(~0, content, length) ^ ~0)) {
				input_err(true, tcm_hcd->pdev->dev.parent,
						"open_short config checksum error\n");
				return -EINVAL;
			}
			zeroflash_hcd->has_open_short_config = true;
			image_info->open_short_config.size = length;
			image_info->open_short_config.data = content;
			image_info->open_short_config.flash_addr = flash_addr;
			input_info(true, tcm_hcd->pdev->dev.parent,
					"open_short config size = %d\n", length);
			input_info(true, tcm_hcd->pdev->dev.parent,
					"open_short config flash address = 0x%08x\n", flash_addr);
		}
	}

	return 0;
}

static int zeroflash_get_fw_image(void)
{
	int retval;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	if (tcm_hcd->force_update)
		goto update;

	if (zeroflash_hcd->fw_entry != NULL)
		return 0;

	if (zeroflash_hcd->image == NULL) {
		retval = request_firmware(&zeroflash_hcd->fw_entry,
				bdata->fw_name, tcm_hcd->pdev->dev.parent);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to request %s\n", bdata->fw_name);
			return retval;
		}
	}

	if (zeroflash_hcd->fw_entry != NULL) { /* add condtion for only prevent */
		input_info(true, tcm_hcd->pdev->dev.parent,
				"Firmware image size = %d\n", (unsigned int)zeroflash_hcd->fw_entry->size);

		zeroflash_hcd->image = zeroflash_hcd->fw_entry->data;
	}
update:
	retval = zeroflash_parse_fw_image();
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to parse firmware image\n");
		release_firmware(zeroflash_hcd->fw_entry);
		zeroflash_hcd->fw_entry = NULL;
		zeroflash_hcd->image = NULL;
		return retval;
	}

	return 0;
}

static void zeroflash_download_config(void)
{
	struct firmware_status *fw_status;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;

	fw_status = &zeroflash_hcd->fw_status;

	if (!fw_status->need_app_config && !fw_status->need_disp_config
			&& !(fw_status->need_open_short_config
			&& zeroflash_hcd->has_open_short_config)
			&& (atomic_read(&tcm_hcd->host_downloading))) {

		pr_info("SAaiOS_TOUCH_DBG: stock download_config skip (need_*=0) queue REINIT host_downloading=1 hdl_version=%u\n",
			fw_status->hdl_version);
		/* Clear host_downloading before queue so HELP_SEND_REINIT
		 * gate (IS_FW_MODE && !host_downloading) can run. Stock
		 * queued first then cleared; helper workqueue still races.
		 */
		atomic_set(&tcm_hcd->host_downloading, 0);
		if (atomic_read(&tcm_hcd->helper.task) == HELP_NONE) {
			atomic_set(&tcm_hcd->helper.task, HELP_SEND_REINIT_NOTIFICATION);
			queue_work(tcm_hcd->helper.workqueue, &tcm_hcd->helper.work);
		}
		return;
	}

	if (atomic_read(&tcm_hcd->host_downloading)) {
		pr_info("SAaiOS_TOUCH_DBG: stock download_config queue config_work need_app=%u need_disp=%u need_osh=%u\n",
			fw_status->need_app_config, fw_status->need_disp_config,
			fw_status->need_open_short_config);
		queue_work(zeroflash_hcd->workqueue, &zeroflash_hcd->config_work);
	}

	return;
}

static int zeroflash_download_open_short_config(void)
{
	int retval;
	unsigned char response_code;
	struct image_info *image_info;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;
	static unsigned int retry_count;

	input_info(true, tcm_hcd->pdev->dev.parent,
			"Downloading open_short config\n");

	image_info = &zeroflash_hcd->image_info;

	if (image_info->open_short_config.size == 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"No open_short config in image file\n");
		return -EINVAL;
	}

	LOCK_BUFFER(zeroflash_hcd->out);

	retval = syna_tcm_alloc_mem(tcm_hcd, &zeroflash_hcd->out,
				image_info->open_short_config.size + 2);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to allocate memory for open_short config\n");
		goto unlock_out;
	}

	switch (zeroflash_hcd->fw_status.hdl_version) {
	case 0:
	case 1:
		zeroflash_hcd->out.buf[0] = 1;
		break;
	case 2:
		zeroflash_hcd->out.buf[0] = 2;
		break;
	default:
		retval = -EINVAL;
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Invalid HDL version (%d)\n", zeroflash_hcd->fw_status.hdl_version);
		goto unlock_out;
	}

	zeroflash_hcd->out.buf[1] = HDL_OPEN_SHORT_CONFIG;

	retval = secure_memcpy(&zeroflash_hcd->out.buf[2],
			zeroflash_hcd->out.buf_size - 2,
			image_info->open_short_config.data,
			image_info->open_short_config.size,
			image_info->open_short_config.size);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to copy open_short config data\n");
		goto unlock_out;
	}

	zeroflash_hcd->out.data_length = image_info->open_short_config.size + 2;

	LOCK_BUFFER(zeroflash_hcd->resp);

	retval = tcm_hcd->write_message(tcm_hcd, CMD_DOWNLOAD_CONFIG,
			zeroflash_hcd->out.buf,	zeroflash_hcd->out.data_length,
			&zeroflash_hcd->resp.buf, &zeroflash_hcd->resp.buf_size,
			&zeroflash_hcd->resp.data_length,
			&response_code,	0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to write command %s\n", STR(CMD_DOWNLOAD_CONFIG));
		if (response_code != STATUS_ERROR)
			goto unlock_resp;
		retry_count++;
		if (DOWNLOAD_RETRY_COUNT && retry_count > DOWNLOAD_RETRY_COUNT)
			goto unlock_resp;
	} else {
		retry_count = 0;
	}

	retval = secure_memcpy((unsigned char *)&zeroflash_hcd->fw_status,
			sizeof(zeroflash_hcd->fw_status),
			zeroflash_hcd->resp.buf,
			zeroflash_hcd->resp.buf_size,
			sizeof(zeroflash_hcd->fw_status));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy firmware status\n");
		goto unlock_resp;
	}

	input_info(true, tcm_hcd->pdev->dev.parent, "open_short config downloaded\n");

	retval = 0;

unlock_resp:
	UNLOCK_BUFFER(zeroflash_hcd->resp);

unlock_out:
	UNLOCK_BUFFER(zeroflash_hcd->out);

	return retval;
}

static int zeroflash_download_disp_config(void)
{
	int retval;
	unsigned char response_code;
	struct image_info *image_info;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;
	static unsigned int retry_count;

	input_info(true, tcm_hcd->pdev->dev.parent,
			"Downloading display config\n");

	image_info = &zeroflash_hcd->image_info;

	if (image_info->disp_config.size == 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"No display config in image file\n");
		return -EINVAL;
	}

	LOCK_BUFFER(zeroflash_hcd->out);

	retval = syna_tcm_alloc_mem(tcm_hcd,
			&zeroflash_hcd->out, image_info->disp_config.size + 2);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to allocate memory for display config\n");
		goto unlock_out;
	}

	switch (zeroflash_hcd->fw_status.hdl_version) {
	case 0:
	case 1:
		zeroflash_hcd->out.buf[0] = 1;
		break;
	case 2:
		zeroflash_hcd->out.buf[0] = 2;
		break;
	default:
		retval = -EINVAL;
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Invalid HDL version (%d)\n", zeroflash_hcd->fw_status.hdl_version);
		goto unlock_out;
	}

	zeroflash_hcd->out.buf[1] = HDL_DISPLAY_CONFIG;

	retval = secure_memcpy(&zeroflash_hcd->out.buf[2],
			zeroflash_hcd->out.buf_size - 2,
			image_info->disp_config.data,
			image_info->disp_config.size,
			image_info->disp_config.size);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to copy display config data\n");
		goto unlock_out;
	}

	zeroflash_hcd->out.data_length = image_info->disp_config.size + 2;

	LOCK_BUFFER(zeroflash_hcd->resp);

	retval = tcm_hcd->write_message(tcm_hcd, CMD_DOWNLOAD_CONFIG,
			zeroflash_hcd->out.buf, zeroflash_hcd->out.data_length,
			&zeroflash_hcd->resp.buf, &zeroflash_hcd->resp.buf_size,
			&zeroflash_hcd->resp.data_length, &response_code, 0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to write command %s\n", STR(CMD_DOWNLOAD_CONFIG));
		if (response_code != STATUS_ERROR)
			goto unlock_resp;
		retry_count++;
		if (DOWNLOAD_RETRY_COUNT && retry_count > DOWNLOAD_RETRY_COUNT)
			goto unlock_resp;
	} else {
		retry_count = 0;
	}

	retval = secure_memcpy((unsigned char *)&zeroflash_hcd->fw_status,
			sizeof(zeroflash_hcd->fw_status),
			zeroflash_hcd->resp.buf,
			zeroflash_hcd->resp.buf_size,
			sizeof(zeroflash_hcd->fw_status));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy firmware status\n");
		goto unlock_resp;
	}

	input_info(true, tcm_hcd->pdev->dev.parent, "Display config downloaded\n");

	retval = 0;

unlock_resp:
	UNLOCK_BUFFER(zeroflash_hcd->resp);

unlock_out:
	UNLOCK_BUFFER(zeroflash_hcd->out);

	return retval;
}

static int zeroflash_download_app_config(void)
{
	int retval;
	unsigned char padding;
	unsigned char response_code;
	struct image_info *image_info;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;
	static unsigned int retry_count;

	input_info(true, tcm_hcd->pdev->dev.parent,
			"Downloading application config\n");

	image_info = &zeroflash_hcd->image_info;

	if (image_info->app_config.size == 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"No application config in image file\n");
		return -EINVAL;
	}

	padding = image_info->app_config.size % 8;
	if (padding)
		padding = 8 - padding;

	LOCK_BUFFER(zeroflash_hcd->out);

	retval = syna_tcm_alloc_mem(tcm_hcd, &zeroflash_hcd->out,
			image_info->app_config.size + 2 + padding);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to allocate memory for application config\n");
		goto unlock_out;
	}

	switch (zeroflash_hcd->fw_status.hdl_version) {
	case 0:
	case 1:
		zeroflash_hcd->out.buf[0] = 1;
		break;
	case 2:
		zeroflash_hcd->out.buf[0] = 2;
		break;
	default:
		retval = -EINVAL;
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Invalid HDL version (%d)\n", zeroflash_hcd->fw_status.hdl_version);
		goto unlock_out;
	}

	zeroflash_hcd->out.buf[1] = HDL_TOUCH_CONFIG;

	retval = secure_memcpy(&zeroflash_hcd->out.buf[2],
			zeroflash_hcd->out.buf_size - 2,
			image_info->app_config.data,
			image_info->app_config.size,
			image_info->app_config.size);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to copy application config data\n");
		goto unlock_out;
	}

	zeroflash_hcd->out.data_length = image_info->app_config.size + 2;
	zeroflash_hcd->out.data_length += padding;

	LOCK_BUFFER(zeroflash_hcd->resp);

	pr_info("SAaiOS_TOUCH_DBG: stock download_app_config write_message cmd=0x%02x len=%u hdl_version=%u HDL_TOUCH_CONFIG=%u\n",
		CMD_DOWNLOAD_CONFIG, zeroflash_hcd->out.data_length,
		zeroflash_hcd->fw_status.hdl_version, HDL_TOUCH_CONFIG);
	{
		unsigned int saved_wr_chunk = tcm_hcd->wr_chunk_size;

		/* Same pattern as 0x45: HDL 0x02 is silent on chunked payload
		 * cmds (LIVE chunked 0x30 512/-62 jammed 0x20). Do not change
		 * 0x45 oneshot. Do not guess hdl_version.
		 */
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x30 force wr_chunk saved=%u forced=0\n",
			syna_tcm_saaios_exp_seq(), saved_wr_chunk);
		tcm_hcd->wr_chunk_size = HDL_WR_CHUNK_SIZE; /* 0 = oneshot */
		retval = tcm_hcd->write_message(tcm_hcd, CMD_DOWNLOAD_CONFIG,
				zeroflash_hcd->out.buf, zeroflash_hcd->out.data_length,
				&zeroflash_hcd->resp.buf, &zeroflash_hcd->resp.buf_size,
				&zeroflash_hcd->resp.data_length, &response_code, 0);
		tcm_hcd->wr_chunk_size = saved_wr_chunk;
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x30 restored wr_chunk=%u (saved=%u)\n",
			syna_tcm_saaios_exp_seq(), tcm_hcd->wr_chunk_size,
			saved_wr_chunk);
	}
	pr_info("SAaiOS_TOUCH_DBG: stock download_app_config write_message retval=%d response_code=0x%02x resp_len=%u\n",
		retval, response_code, zeroflash_hcd->resp.data_length);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to write command %s\n", STR(CMD_DOWNLOAD_CONFIG));
		if (response_code != STATUS_ERROR)
			goto unlock_resp;
		retry_count++;
		if (DOWNLOAD_RETRY_COUNT && retry_count > DOWNLOAD_RETRY_COUNT)
			goto unlock_resp;
	} else {
		retry_count = 0;
	}

	retval = secure_memcpy((unsigned char *)&zeroflash_hcd->fw_status,
			sizeof(zeroflash_hcd->fw_status),
			zeroflash_hcd->resp.buf,
			zeroflash_hcd->resp.buf_size,
			sizeof(zeroflash_hcd->fw_status));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy firmware status\n");
		goto unlock_resp;
	}

	input_info(true, tcm_hcd->pdev->dev.parent, "Application config downloaded\n");

	retval = 0;

unlock_resp:
	UNLOCK_BUFFER(zeroflash_hcd->resp);

unlock_out:
	UNLOCK_BUFFER(zeroflash_hcd->out);

	return retval;
}

static void zeroflash_download_config_work(struct work_struct *work)
{
	int retval;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;

	retval = zeroflash_get_fw_image();
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to get firmware image\n");
		return;
	}

	input_info(true, tcm_hcd->pdev->dev.parent, "Start of config download\n");

	if (zeroflash_hcd->fw_status.need_app_config) {
		retval = zeroflash_download_app_config();
		if (retval < 0) {
			atomic_set(&tcm_hcd->host_downloading, 0);
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to download application config, abort\n");
			return;
		}
		goto exit;
	}

	if (zeroflash_hcd->fw_status.need_disp_config) {
		retval = zeroflash_download_disp_config();
		if (retval < 0) {
			atomic_set(&tcm_hcd->host_downloading, 0);
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to download display config, abort\n");
			return;
		}
		goto exit;
	}

	if (zeroflash_hcd->fw_status.need_open_short_config &&
			zeroflash_hcd->has_open_short_config) {

		retval = zeroflash_download_open_short_config();
		if (retval < 0) {
			atomic_set(&tcm_hcd->host_downloading, 0);
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to download open_short config, abort\n");
			return;
		}
		goto exit;
	}

exit:
	input_info(true, tcm_hcd->pdev->dev.parent, "End of config download\n");

	zeroflash_download_config();

	return;
}

int zeroflash_saaios_download_app_config(void)
{
	int retval;
	struct firmware_status *st;

	if (!zeroflash_hcd)
		return -ENODEV;
	if (zeroflash_hcd->image_info.app_config.size == 0) {
		retval = zeroflash_get_fw_image();
		if (retval < 0)
			return retval;
	}
	/* Bypass need_app_config only. Stock zeroflash_download_app_config()
	 * builds CMD_DOWNLOAD_CONFIG from leftover 0x1b fw_status.hdl_version
	 * (0/1 → type byte 1; 2 → type byte 2). Do not guess hdl_version=2.
	 * write_message is oneshot (wr_chunk=0) inside that function.
	 */
	st = &zeroflash_hcd->fw_status;
	pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x30 stock zeroflash_download_app_config hdl_version=%u need_app=%u need_disp=%u need_osh=%u (bypass need_app only, no version guess)\n",
		syna_tcm_saaios_exp_seq(), st->hdl_version, st->need_app_config,
		st->need_disp_config, st->need_open_short_config);
	retval = zeroflash_download_app_config();
	st = &zeroflash_hcd->fw_status;
	pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x30 fw_status after retval=%d need_app=%u need_disp=%u need_osh=%u hdl_version=%u invalid_static=%u\n",
		syna_tcm_saaios_exp_seq(), retval, st->need_app_config,
		st->need_disp_config, st->need_open_short_config,
		st->hdl_version, st->invalid_static_config);
	return retval;
}

static int zeroflash_download_app_fw(void)
{
	int retval;
	unsigned char command;
	struct image_info *image_info;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;
#if TP_RESET_TO_HDL_DELAY_MS
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;
#endif

	input_info(true, tcm_hcd->pdev->dev.parent,
			"Downloading application firmware\n");

	image_info = &zeroflash_hcd->image_info;

	if (image_info->app_firmware.size == 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"No application firmware in image file\n");
		return -EINVAL;
	}

	LOCK_BUFFER(zeroflash_hcd->out);

	retval = syna_tcm_alloc_mem(tcm_hcd,
			&zeroflash_hcd->out,
			image_info->app_firmware.size);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to allocate memory for application firmware\n");
		UNLOCK_BUFFER(zeroflash_hcd->out);
		return retval;
	}

	retval = secure_memcpy(zeroflash_hcd->out.buf,
			zeroflash_hcd->out.buf_size,
			image_info->app_firmware.data,
			image_info->app_firmware.size,
			image_info->app_firmware.size);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to copy application firmware data\n");
		UNLOCK_BUFFER(zeroflash_hcd->out);
		return retval;
	}

	zeroflash_hcd->out.data_length = image_info->app_firmware.size;

	command = F35_WRITE_FW_TO_PMEM_COMMAND;

#if TP_RESET_TO_HDL_DELAY_MS
	if (bdata->tpio_reset_gpio >= 0) {
		gpio_set_value(bdata->tpio_reset_gpio, bdata->reset_on_state);
		msleep(bdata->reset_active_ms);
		gpio_set_value(bdata->tpio_reset_gpio, !bdata->reset_on_state);
		mdelay(TP_RESET_TO_HDL_DELAY_MS);
	}
#endif

	retval = syna_tcm_rmi_write(tcm_hcd,
			zeroflash_hcd->f35_addr.control_base + F35_CTRL3_OFFSET,
			&command, sizeof(command));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to write F$35 command\n");
		UNLOCK_BUFFER(zeroflash_hcd->out);
		return retval;
	}

	retval = syna_tcm_rmi_write(tcm_hcd,
			zeroflash_hcd->f35_addr.control_base + F35_CTRL7_OFFSET,
			zeroflash_hcd->out.buf, zeroflash_hcd->out.data_length);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to write application firmware data\n");
		UNLOCK_BUFFER(zeroflash_hcd->out);
		return retval;
	}

	UNLOCK_BUFFER(zeroflash_hcd->out);

	input_info(true, tcm_hcd->pdev->dev.parent, "Application firmware downloaded\n");

	return 0;
}


static void zeroflash_do_f35_firmware_download(void)
{
	int retval = 0;
	struct rmi_f35_data data;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;
	static unsigned int retry_count;

	if (tcm_hcd->irq_enabled) {
		retval = tcm_hcd->enable_irq(tcm_hcd, false, true);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to disable interrupt\n");
		}
	}

	input_info(true, tcm_hcd->pdev->dev.parent,
			"Prepare F35 firmware download\n");

	if (tcm_hcd->id_info.mode == MODE_ROMBOOTLOADER) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Incorrect uboot type, exit\n");
		goto exit;
	}
	retval = zeroflash_check_uboot();
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to find valid uboot\n");
		goto exit;
	}

	atomic_set(&tcm_hcd->host_downloading, 1);

	retval = syna_tcm_rmi_read(tcm_hcd, zeroflash_hcd->f35_addr.data_base,
			(unsigned char *)&data, sizeof(data));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to read F$35 data\n");
		goto exit;
	}

	if (data.error_code != REQUESTING_FIRMWARE) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Microbootloader error code = 0x%02x\n", data.error_code);
		if (data.error_code != CHECKSUM_FAILURE) {
			retval = -EIO;
			goto exit;
		} else {
			retry_count++;
		}
	} else {
		retry_count = 0;
	}

	retval = zeroflash_get_fw_image();
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to get firmware image\n");
		goto exit;
	}

	input_info(true, tcm_hcd->pdev->dev.parent, "Start of firmware download\n");

	/* perform firmware downloading */
	retval = zeroflash_download_app_fw();
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to download application firmware\n");
		goto exit;
	}

	input_info(true, tcm_hcd->pdev->dev.parent, "End of firmware download\n");

exit:
	if (retval < 0)
		retry_count++;

	if (DOWNLOAD_RETRY_COUNT && retry_count > DOWNLOAD_RETRY_COUNT) {
		retval = tcm_hcd->enable_irq(tcm_hcd, false, true);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to disable interrupt\n");
		}

		input_dbg(true, tcm_hcd->pdev->dev.parent, "Interrupt is disabled\n");
	} else {
		retval = tcm_hcd->enable_irq(tcm_hcd, true, NULL);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to enable interrupt\n");
		}
	}

	return;
}

static void zeroflash_do_romboot_firmware_download(void)
{
	int retval;
	unsigned char *out_buf = NULL;
	unsigned char *resp_buf = NULL;
	unsigned int resp_buf_size;
	unsigned int resp_length;
	unsigned int data_size_blocks;
	unsigned int image_size;
	struct syna_tcm_hcd *tcm_hcd = zeroflash_hcd->tcm_hcd;

	input_info(true, tcm_hcd->pdev->dev.parent,
			"Prepare ROMBOOT firmware download\n");

	atomic_set(&tcm_hcd->host_downloading, 1);
	atomic_set(&hdl_status_seen, 0);
	syna_tcm_saaios_allow_hdl_reinit(0);
	syna_tcm_saaios_reset_hdl_observe();
	resp_buf = NULL;
	resp_buf_size = 0;
	resp_length = 0; /* since59 printed garbage 1714697008 on IDENTIFY-abort */

	if (!tcm_hcd->irq_enabled) {
		retval = tcm_hcd->enable_irq(tcm_hcd, true, NULL);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to enable interrupt\n");
		}
	}

	pm_stay_awake(&tcm_hcd->pdev->dev);

	if (tcm_hcd->id_info.mode != MODE_ROMBOOTLOADER) {
		if (tcm_hcd->id_info.mode == MODE_HOSTDOWNLOAD_FIRMWARE)
			pr_info("SAaiOS_TOUCH_DBG: CONTAMINATED: mode already 0x02 before 0x45 -- do not send 0x45\n");
		input_err(true, tcm_hcd->pdev->dev.parent, "Not in romboot mode\n");
		atomic_set(&tcm_hcd->host_downloading, 0);
		goto exit;
	}

	retval = zeroflash_get_fw_image();
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to request romboot.img\n");
		goto exit;
	}

	image_size = (unsigned int)zeroflash_hcd->image_info.app_firmware.size;

	input_dbg(true, tcm_hcd->pdev->dev.parent, "image_size = %d\n", image_size);

	data_size_blocks = image_size / 16;

	out_buf = vzalloc(image_size + RESERVED_BYTES);
	if (!out_buf) {
		input_err(true, tcm_hcd->pdev->dev.parent, "out_buf kzalloc\n");
		goto exit;
	}

	memset(out_buf, 0x00, RESERVED_BYTES);

	out_buf[0] = zeroflash_hcd->image_info.app_firmware.size >> 16;

	retval = secure_memcpy(&out_buf[RESERVED_BYTES],
			zeroflash_hcd->image_info.app_firmware.size,
			zeroflash_hcd->image_info.app_firmware.data,
			zeroflash_hcd->image_info.app_firmware.size,
			zeroflash_hcd->image_info.app_firmware.size);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to copy payload\n");
		goto error_secure_memcpy;
	}

	input_dbg(true, tcm_hcd->pdev->dev.parent,
			"data_size_blocks: %d\n", data_size_blocks);

	{
		char sha_hex[65];
		unsigned int file_off = 0;
		unsigned int ram_crc;
		int match;
		unsigned char *app_code = &out_buf[RESERVED_BYTES];

		if (zeroflash_hcd->fw_entry &&
				zeroflash_hcd->image_info.app_firmware.data &&
				zeroflash_hcd->fw_entry->data)
			file_off = (unsigned int)(zeroflash_hcd->image_info.app_firmware.data -
					zeroflash_hcd->fw_entry->data);

		pr_info("SAaiOS_TOUCH_DBG: SAAIOS_FORCE_CORRUPT_APP=%d syna_corrupt_app=%d (cmdline/module_param; since60 BL dropped token) cmdline='%s'\n",
			SAAIOS_FORCE_CORRUPT_APP, syna_corrupt_app,
			saved_command_line ? saved_command_line : "(null)");

		syna_sha256_hex(app_code, image_size, sha_hex, sizeof(sha_hex));
		match = sha_hex[0] && !strcmp(sha_hex, SAAIOS_APP_CODE_SHA256_GOOD);
		pr_info("SAaiOS_TOUCH_DBG: APP_CODE RAM sha256=%s match=%d file_off=%u size=%u (file copy, since64 expect 1 size=%u)\n",
			sha_hex, match, file_off, image_size, SAAIOS_APP_CODE_WIPE_SIZE);

		if (SAAIOS_FORCE_CORRUPT_APP) {
			pr_info("SAaiOS_TOUCH_DBG: CORRUPT_FAILED abort 0x45 (since64 must send unmodified APP_CODE)\n");
			goto error_secure_memcpy;
		}
		pr_info("SAaiOS_TOUCH_DBG: SAAIOS_FORCE_CORRUPT_APP=%d syna_corrupt_app=%d (RAM APP_CODE unmodified) file_off=%u\n",
			SAAIOS_FORCE_CORRUPT_APP, syna_corrupt_app, file_off);

		syna_sha256_hex(app_code, image_size, sha_hex, sizeof(sha_hex));
		match = sha_hex[0] && !strcmp(sha_hex, SAAIOS_APP_CODE_SHA256_GOOD);
		ram_crc = crc32(~0, app_code, image_size) ^ ~0;
		pr_info("SAaiOS_TOUCH_DBG: APP_CODE[0..15]=%02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x (expect 55 aa 01 00) file_off=%u size=%u\n",
			app_code[0], app_code[1], app_code[2], app_code[3],
			app_code[4], app_code[5], app_code[6], app_code[7],
			app_code[8], app_code[9], app_code[10], app_code[11],
			app_code[12], app_code[13], app_code[14], app_code[15],
			file_off, image_size);
		pr_info("SAaiOS_TOUCH_DBG: reserved[0..13]=%02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x reserved[0]=size>>16 expect 0x01\n",
			out_buf[0], out_buf[1], out_buf[2], out_buf[3],
			out_buf[4], out_buf[5], out_buf[6], out_buf[7],
			out_buf[8], out_buf[9], out_buf[10], out_buf[11],
			out_buf[12], out_buf[13]);
		pr_info("SAaiOS_TOUCH_DBG: APP_CODE ram sha256=%s match=%d known_good=%s (since64 expect match=1)\n",
			sha_hex, match, SAAIOS_APP_CODE_SHA256_GOOD);
		pr_info("SAaiOS_TOUCH_DBG: APP_CODE ram crc32=0x%08x known_good=0x%08x APP_CODE[0]=0x%02x APP_CODE[%u]=0x%02x\n",
			ram_crc, SAAIOS_APP_CODE_CRC32_GOOD, app_code[0],
			image_size ? image_size - 1 : 0,
			image_size ? app_code[image_size - 1] : 0xff);
		pr_info("SAaiOS_TOUCH_DBG: 0x45 payload APP_CODE[0]=0x%02x (write_message payload=%p app_code=%p, expect 0x55)\n",
			app_code[0], out_buf, app_code);
		if (!match || app_code[0] != 0x55 || app_code[1] != 0xaa ||
				app_code[2] != 0x01 || app_code[3] != 0x00) {
			pr_info("SAaiOS_TOUCH_DBG: abort 0x45 (since64 requires unmodified APP_CODE sha256 match=1 header 55 aa 01 00)\n");
			goto error_secure_memcpy;
		}
	}

	syna_tcm_dump_identify(tcm_hcd, NULL, 0,
		"immediately before 0x45 (cached id_info, not a live Identify)");
	{
		unsigned int max_write_size = le2_to_uint(tcm_hcd->id_info.max_write_size);
		unsigned int saved_wr_chunk = tcm_hcd->wr_chunk_size;

		pr_info("SAaiOS_TOUCH_DBG: wr_chunk before 0x45=%u max_write=%u WR_CHUNK_SIZE=%u HDL_WR_CHUNK_SIZE=%u (expect 512 = MIN(max_write, WR_CHUNK))\n",
			saved_wr_chunk, max_write_size, WR_CHUNK_SIZE, HDL_WR_CHUNK_SIZE);
		tcm_hcd->wr_chunk_size = HDL_WR_CHUNK_SIZE; /* 0 = one-shot */
		pr_info("SAaiOS_TOUCH_DBG: 0x45 force wr_chunk saved=%u forced=HDL_WR_CHUNK_SIZE=%u (0=oneshot continuous)\n",
			saved_wr_chunk, tcm_hcd->wr_chunk_size);
		pr_info("SAaiOS_TOUCH_DBG: 0x45 start stock OSS write_message payload=%u reserved=%u app=%u mode=0x%02x host_downloading=%d fb_ready=%u wr_chunk=%u\n",
			image_size + RESERVED_BYTES, RESERVED_BYTES, image_size,
			tcm_hcd->id_info.mode, atomic_read(&tcm_hcd->host_downloading),
			tcm_hcd->fb_ready, tcm_hcd->wr_chunk_size);

		retval = tcm_hcd->write_message(tcm_hcd, CMD_ROMBOOT_DOWNLOAD,
				out_buf, image_size + RESERVED_BYTES, &resp_buf,
				&resp_buf_size, &resp_length, NULL, 20);
		tcm_hcd->wr_chunk_size = saved_wr_chunk;
		pr_info("SAaiOS_TOUCH_DBG: 0x45 restored wr_chunk=%u (saved=%u)\n",
			tcm_hcd->wr_chunk_size, saved_wr_chunk);
	}
	{
		int attn = -1;

		if (tcm_hcd->hw_if && tcm_hcd->hw_if->bdata &&
				tcm_hcd->hw_if->bdata->irq_gpio >= 0)
			attn = gpio_get_value(tcm_hcd->hw_if->bdata->irq_gpio);
		pr_info("SAaiOS_TOUCH_DBG: 0x45 write_message retval=%d resp_len=%u response_code=0x%02x mode=0x%02x part='%s' packrat=%u host_downloading=%d ATTN gpio=%d\n",
			retval, resp_length, tcm_hcd->response_code, tcm_hcd->id_info.mode,
			tcm_hcd->id_info.part_number, tcm_hcd->packrat_number,
			atomic_read(&tcm_hcd->host_downloading), attn);
	}
	syna_tcm_dump_identify(tcm_hcd, NULL, 0, "after 0x45 write_message (id_info)");
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to write command ROMBOOT DOWNLOAD");
		pr_info("SAaiOS_TOUCH_DBG: 0x45 path return before stock switch_mode (write_message failed)\n");
		goto error_secure_memcpy;
	}

	/* Stock OSS: switch_mode(BOOTLOADER) after successful 0x45.
	 * From still-0x04 this is 0x42; from 0x02 it is 0x1f (since67
	 * timeout). MODE_APPLICATION_FIRMWARE is 0x01; live IDENTIFY
	 * after 0x45 is MODE_HOSTDOWNLOAD_FIRMWARE (0x02). Skip 0x1f.
	 * Log is "HDL firmware running" — 0x02 is not application (0x01).
	 */
	if (tcm_hcd->id_info.mode == MODE_HOSTDOWNLOAD_FIRMWARE) {
		pr_info("SAaiOS_TOUCH_DBG: skip switch_mode: HDL firmware running after 0x45 mode=0x%02x part='%s' packrat=%u\n",
			tcm_hcd->id_info.mode, tcm_hcd->id_info.part_number,
			tcm_hcd->packrat_number);
		goto error_secure_memcpy;
	}

	pr_info("SAaiOS_TOUCH_DBG: stock switch_mode enter after 0x45 write_message retval=%d mode=0x%02x part='%s' packrat=%u (0x42 if RomBoot 0x04)\n",
		retval, tcm_hcd->id_info.mode, tcm_hcd->id_info.part_number,
		tcm_hcd->packrat_number);
	retval = tcm_hcd->switch_mode(tcm_hcd, FW_MODE_BOOTLOADER);
	pr_info("SAaiOS_TOUCH_DBG: stock switch_mode exit retval=%d mode=0x%02x part='%s' packrat=%u\n",
		retval, tcm_hcd->id_info.mode, tcm_hcd->id_info.part_number,
		tcm_hcd->packrat_number);
	syna_tcm_dump_identify(tcm_hcd, NULL, 0, "after stock switch_mode (id_info)");
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to switch to bootloader");
		goto error_secure_memcpy;
	}

error_secure_memcpy:
	vfree(out_buf);
exit:
	pm_relax(&tcm_hcd->pdev->dev);
	return;
}

#define SAAIOS_PANEL_WAIT_MS		3000
#define SAAIOS_PANEL_POLL_MS		25
#define SAAIOS_POST_PANEL_SLEEP_MS	400

static const char *syna_rail_str(int v)
{
	if (v == -2)
		return "absent";
	if (v < 0)
		return "err";
	return v ? "1" : "0";
}

static int syna_reg_is_enabled(struct regulator *reg)
{
	int v;

	if (!reg || IS_ERR(reg))
		return -2;
	v = regulator_is_enabled(reg);
	if (v < 0)
		return v;
	return v ? 1 : 0;
}

static int syna_attn_gpio(struct syna_tcm_hcd *tcm_hcd)
{
	if (!tcm_hcd->hw_if || !tcm_hcd->hw_if->bdata ||
			tcm_hcd->hw_if->bdata->irq_gpio < 0)
		return -2;
	return gpio_get_value(tcm_hcd->hw_if->bdata->irq_gpio);
}

static bool syna_rail_ready(int v)
{
	/* 1 = on. absent/err: do not block. 0 = not ready. */
	return v != 0;
}

static bool syna_attn_idle(struct syna_tcm_hcd *tcm_hcd, int attn)
{
	int on = 0;

	if (attn < 0)
		return true;
	if (tcm_hcd->hw_if && tcm_hcd->hw_if->bdata)
		on = tcm_hcd->hw_if->bdata->irq_on_state;
	return attn != on;
}

static void syna_log_panel_state(struct syna_tcm_hcd *tcm_hcd, const char *when,
		int vdd, int rst, int bl, int attn)
{
	int irq_on = 0;

	if (tcm_hcd->hw_if && tcm_hcd->hw_if->bdata)
		irq_on = tcm_hcd->hw_if->bdata->irq_on_state;
	pr_info("SAaiOS_TOUCH_DBG: %s fb_ready=%u vdd_ldo28=%s gpio_lcd_rst=%s gpio_lcd_bl_en=%s attn_gpio=%d irq_on_state=%d mode=0x%02x part='%s'\n",
		when, tcm_hcd->fb_ready,
		syna_rail_str(vdd), syna_rail_str(rst), syna_rail_str(bl),
		attn, irq_on, tcm_hcd->id_info.mode,
		tcm_hcd->id_info.part_number);
}

#define SAAIOS_ROM_PACKRAT		2893283U
#define SAAIOS_ROM_PART			"td4150_rom-10.0"
#define SAAIOS_ONCHIP_PACKRAT		2100027192U
#define SAAIOS_ONCHIP_PART		"TD4150-12.0.12"

static atomic_t saaios_panel_cb_n = ATOMIC_INIT(0);

static u64 syna_boot_ns(void)
{
	return ktime_get_boot_ns();
}

static void syna_part_copy(struct syna_tcm_hcd *tcm_hcd, char *part, size_t n)
{
	memset(part, 0, n);
	if (!tcm_hcd || n < 2)
		return;
	if (n > 16)
		memcpy(part, tcm_hcd->id_info.part_number, 16);
	else if (n > 1)
		memcpy(part, tcm_hcd->id_info.part_number, n - 1);
}

static bool syna_identify_onchip_hdl(struct syna_tcm_hcd *tcm_hcd)
{
	char part[17];

	syna_part_copy(tcm_hcd, part, sizeof(part));
	return tcm_hcd->id_info.mode == MODE_HOSTDOWNLOAD_FIRMWARE ||
		tcm_hcd->packrat_number == SAAIOS_ONCHIP_PACKRAT ||
		!strncmp(part, SAAIOS_ONCHIP_PART, sizeof(part));
}

static bool syna_identify_romboot_ok(struct syna_tcm_hcd *tcm_hcd)
{
	char part[17];

	syna_part_copy(tcm_hcd, part, sizeof(part));
	return tcm_hcd->id_info.mode == MODE_ROMBOOTLOADER &&
		tcm_hcd->packrat_number == SAAIOS_ROM_PACKRAT &&
		!strncmp(part, SAAIOS_ROM_PART, sizeof(part));
}

static void syna_log_contaminated_before_0x45(struct syna_tcm_hcd *tcm_hcd,
		const char *when)
{
	char part[17];

	syna_part_copy(tcm_hcd, part, sizeof(part));
	pr_info("SAaiOS_TOUCH_DBG: CONTAMINATED: on-chip HDL already before 0x45 (%s) part='%s' mode=0x%02x packrat=%u -- do not send 0x45 (panel delay is not the cause)\n",
		when ? when : "-", part, tcm_hcd->id_info.mode,
		tcm_hcd->packrat_number);
}

/* Hash APP_CODE before delayed 0x45 start. 1 = match and header ok. */
static int syna_pre45_check_app_code(struct syna_tcm_hcd *tcm_hcd)
{
	char sha_hex[65];
	const unsigned char *app;
	unsigned int size;
	int match, header_ok, retval;

	retval = zeroflash_get_fw_image();
	if (retval < 0) {
		pr_info("SAaiOS_TOUCH_DBG: abort delayed 0x45 (no fw image retval=%d)\n",
			retval);
		return 0;
	}
	app = zeroflash_hcd->image_info.app_firmware.data;
	size = (unsigned int)zeroflash_hcd->image_info.app_firmware.size;
	if (!app || size < 4) {
		pr_info("SAaiOS_TOUCH_DBG: abort delayed 0x45 (APP_CODE missing size=%u)\n",
			size);
		return 0;
	}

	pr_info("SAaiOS_TOUCH_DBG: APP_CODE[0..3]=%02x %02x %02x %02x (expect 55 aa 01 00) size=%u\n",
		app[0], app[1], app[2], app[3], size);
	header_ok = (app[0] == 0x55 && app[1] == 0xaa &&
			app[2] == 0x01 && app[3] == 0x00);

	memset(sha_hex, 0, sizeof(sha_hex));
	syna_sha256_hex(app, size, sha_hex, sizeof(sha_hex));
	match = sha_hex[0] && strcmp(sha_hex, "alloc-fail") &&
		strcmp(sha_hex, "digest-fail") && strcmp(sha_hex, "null") &&
		!strcmp(sha_hex, SAAIOS_APP_CODE_SHA256_GOOD);
	pr_info("SAaiOS_TOUCH_DBG: APP_CODE file sha256=%s\n", sha_hex);
	pr_info("SAaiOS_TOUCH_DBG: sha256 match=%d known_good=%s (file, expect 1 before RAM flip)\n",
		match, SAAIOS_APP_CODE_SHA256_GOOD);

	if (!match && header_ok) {
		pr_info("SAaiOS_TOUCH_DBG: SHA bug: match=0 but APP_CODE[0..3]=55 aa 01 00 -- digest printer/API still broken (since57 printed 16 zero bytes + tail of known_good %s). Do not send 0x45.\n",
			SAAIOS_APP_CODE_SHA256_GOOD);
		if (strlen(sha_hex) >= 64 &&
				!strncmp(sha_hex,
					"00000000000000000000000000000000", 32) &&
				!strcmp(&sha_hex[32],
					&SAAIOS_APP_CODE_SHA256_GOOD[32]))
			pr_info("SAaiOS_TOUCH_DBG: SHA bug pattern=16 zero digest bytes + known_good tail (since57 printer)\n");
		return 0;
	}
	if (!header_ok || !match) {
		pr_info("SAaiOS_TOUCH_DBG: abort delayed 0x45 APP_CODE header_ok=%d sha256 match=%d\n",
			header_ok, match);
		return 0;
	}
	return 1;
}

static void zeroflash_delayed_romboot_work(struct work_struct *work)
{
	struct syna_tcm_hcd *tcm_hcd;
	int vdd, rst, bl, attn;
	unsigned int waited = 0;
	unsigned int n;
	s64 elapsed;
	u64 now_ns;
	bool ready = false;
	char part[17];

	(void)work;
	if (!zeroflash_hcd)
		return;
	tcm_hcd = zeroflash_hcd->tcm_hcd;
	if (!tcm_hcd)
		return;

	n = zeroflash_hcd->panel_cb_n;
	now_ns = syna_boot_ns();
	pr_info("SAaiOS_TOUCH_DBG: delayed 0x45 work enter (0x45 not started) n=%u ktime_boot_ns=%llu ktime_boot_ms=%llu fb_ready=%u mode=0x%02x panel_cb_count=%u\n",
		n, now_ns, now_ns / 1000000ULL, tcm_hcd->fb_ready,
		tcm_hcd->id_info.mode,
		(unsigned int)atomic_read(&saaios_panel_cb_n));

	while (waited <= SAAIOS_PANEL_WAIT_MS) {
		vdd = syna_reg_is_enabled(tcm_hcd->regulator_vdd);
		rst = syna_reg_is_enabled(tcm_hcd->regulator_lcd_reset);
		bl = syna_reg_is_enabled(tcm_hcd->regulator_lcd_bl_en);
		attn = syna_attn_gpio(tcm_hcd);

		if (syna_identify_onchip_hdl(tcm_hcd) ||
				tcm_hcd->id_info.mode != MODE_ROMBOOTLOADER) {
			syna_log_panel_state(tcm_hcd,
				"delayed 0x45 abort contaminated during wait",
				vdd, rst, bl, attn);
			syna_log_contaminated_before_0x45(tcm_hcd, "during rail wait");
			return;
		}

		if (tcm_hcd->fb_ready && syna_rail_ready(vdd) &&
				syna_rail_ready(rst) && syna_rail_ready(bl) &&
				syna_attn_idle(tcm_hcd, attn)) {
			ready = true;
			break;
		}
		if (waited == 0 || (waited % 500) == 0)
			syna_log_panel_state(tcm_hcd,
				"delayed 0x45 waiting panel rails",
				vdd, rst, bl, attn);
		msleep(SAAIOS_PANEL_POLL_MS);
		waited += SAAIOS_PANEL_POLL_MS;
	}

	vdd = syna_reg_is_enabled(tcm_hcd->regulator_vdd);
	rst = syna_reg_is_enabled(tcm_hcd->regulator_lcd_reset);
	bl = syna_reg_is_enabled(tcm_hcd->regulator_lcd_bl_en);
	attn = syna_attn_gpio(tcm_hcd);
	syna_log_panel_state(tcm_hcd,
		ready ? "delayed 0x45 rails ready" :
			"delayed 0x45 wait timeout (continue if still 0x04)",
		vdd, rst, bl, attn);

	pr_info("SAaiOS_TOUCH_DBG: delayed 0x45 additional sleep %u ms after panel conditions waited=%u ready=%d\n",
		SAAIOS_POST_PANEL_SLEEP_MS, waited, ready ? 1 : 0);
	msleep(SAAIOS_POST_PANEL_SLEEP_MS);

	vdd = syna_reg_is_enabled(tcm_hcd->regulator_vdd);
	rst = syna_reg_is_enabled(tcm_hcd->regulator_lcd_reset);
	bl = syna_reg_is_enabled(tcm_hcd->regulator_lcd_bl_en);
	attn = syna_attn_gpio(tcm_hcd);
	now_ns = syna_boot_ns();
	elapsed = ktime_to_ms(ktime_sub(ktime_get(),
				zeroflash_hcd->panel_exit_ktime));
	if (zeroflash_hcd->panel_exit_boot_ns &&
			now_ns >= zeroflash_hcd->panel_exit_boot_ns)
		elapsed = (s64)((now_ns - zeroflash_hcd->panel_exit_boot_ns) /
				1000000ULL);

	pr_info("SAaiOS_TOUCH_DBG: elapsed since panel exit = %lld ms ktime_boot_ns=%llu panel_exit_boot_ns=%llu n=%u panel_cb_count=%u fb_ready=%u waited=%u\n",
		elapsed, now_ns, zeroflash_hcd->panel_exit_boot_ns, n,
		(unsigned int)atomic_read(&saaios_panel_cb_n),
		tcm_hcd->fb_ready, waited);

	syna_part_copy(tcm_hcd, part, sizeof(part));
	pr_info("SAaiOS_TOUCH_DBG: IDENTIFY immediately before delayed 0x45: part=%s mode=0x%02x packrat=%u\n",
		part, tcm_hcd->id_info.mode, tcm_hcd->packrat_number);
	syna_tcm_dump_identify(tcm_hcd, NULL, 0,
		"immediately before delayed 0x45 (cached id_info, no 0x40/0x02 cmd)");

	if (syna_identify_onchip_hdl(tcm_hcd)) {
		syna_log_contaminated_before_0x45(tcm_hcd,
			"IDENTIFY immediately before delayed 0x45");
		return;
	}
	if (!syna_identify_romboot_ok(tcm_hcd)) {
		pr_info("SAaiOS_TOUCH_DBG: abort delayed 0x45 IDENTIFY not RomBoot 0x04 td4150_rom-10.0 packrat 2893283 (part='%s' mode=0x%02x packrat=%u)\n",
			part, tcm_hcd->id_info.mode, tcm_hcd->packrat_number);
		return;
	}

	if (!syna_pre45_check_app_code(tcm_hcd))
		return;

	now_ns = syna_boot_ns();
	pr_info("SAaiOS_TOUCH_DBG: delayed 0x45 start n=%u ktime_boot_ns=%llu ktime_boot_ms=%llu elapsed since panel exit = %lld ms fb_ready=%u waited=%u panel_cb_count=%u\n",
		n, now_ns, now_ns / 1000000ULL, elapsed, tcm_hcd->fb_ready,
		waited, (unsigned int)atomic_read(&saaios_panel_cb_n));
	syna_log_panel_state(tcm_hcd, "delayed 0x45 start", vdd, rst, bl, attn);

	tcm_hcd->romboot_download_deferred = false;
	pr_info("SAaiOS_TOUCH_DBG: panel callback finished, run stock OSS zeroflash_do_romboot_firmware_download mode=0x%02x fb_ready=%u\n",
		tcm_hcd->id_info.mode, tcm_hcd->fb_ready);
	zeroflash_do_romboot_firmware_download();
}

int zeroflash_on_panel_enabled(struct syna_tcm_hcd *tcm_hcd)
{
	static bool scheduled;
	unsigned int n;
	u64 ns;

	if (!tcm_hcd)
		return -EINVAL;

	n = (unsigned int)atomic_inc_return(&saaios_panel_cb_n);
	ns = syna_boot_ns();
	pr_info("SAaiOS_TOUCH_DBG: panel callback enter n=%u ktime_boot_ns=%llu ktime_boot_ms=%llu fb_ready=%u deferred=%d mode=0x%02x scheduled=%d (0x45 not started)\n",
		n, ns, ns / 1000000ULL, tcm_hcd->fb_ready,
		tcm_hcd->romboot_download_deferred ? 1 : 0,
		tcm_hcd->id_info.mode, scheduled ? 1 : 0);

	if (scheduled) {
		ns = syna_boot_ns();
		pr_info("SAaiOS_TOUCH_DBG: panel callback exit n=%u ktime_boot_ns=%llu ktime_boot_ms=%llu (already scheduled, 0x45 not started) fb_ready=%u\n",
			n, ns, ns / 1000000ULL, tcm_hcd->fb_ready);
		return 0;
	}
	if (!tcm_hcd->romboot_download_deferred) {
		ns = syna_boot_ns();
		pr_info("SAaiOS_TOUCH_DBG: panel callback exit n=%u ktime_boot_ns=%llu ktime_boot_ms=%llu (not deferred, 0x45 not started)\n",
			n, ns, ns / 1000000ULL);
		return 0;
	}
	if (!zeroflash_hcd || !zeroflash_hcd->workqueue) {
		ns = syna_boot_ns();
		pr_info("SAaiOS_TOUCH_DBG: panel callback exit n=%u ktime_boot_ns=%llu ktime_boot_ms=%llu abort (no zeroflash workqueue)\n",
			n, ns, ns / 1000000ULL);
		return -EINVAL;
	}

	scheduled = true;
	/* Keep romboot_download_deferred until delayed work runs 0x45. */
	zeroflash_hcd->panel_cb_n = n;
	zeroflash_hcd->panel_exit_ktime = ktime_get();
	zeroflash_hcd->panel_exit_boot_ns = syna_boot_ns();
	queue_delayed_work(zeroflash_hcd->workqueue,
			&zeroflash_hcd->romboot_delay_work, 0);
	ns = syna_boot_ns();
	pr_info("SAaiOS_TOUCH_DBG: panel callback exit n=%u ktime_boot_ns=%llu ktime_boot_ms=%llu (delayed 0x45 queued, 0x45 not started) fb_ready=%u mode=0x%02x\n",
		n, ns, ns / 1000000ULL, tcm_hcd->fb_ready, tcm_hcd->id_info.mode);
	return 0;
}

int zeroflash_init(struct syna_tcm_hcd *tcm_hcd)
{
	int retval = 0;
	int idx;

	zeroflash_hcd = NULL;
	if (!(tcm_hcd->in_hdl_mode))
		return 0;

	zeroflash_hcd = kzalloc(sizeof(*zeroflash_hcd), GFP_KERNEL);
	if (!zeroflash_hcd) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to allocate memory for zeroflash_hcd\n");
		return -ENOMEM;
	}

	zeroflash_hcd->tcm_hcd = tcm_hcd;
	zeroflash_hcd->image = NULL;
	zeroflash_hcd->has_hdl = false;
	zeroflash_hcd->f35_ready = false;
	zeroflash_hcd->has_open_short_config = false;

	INIT_BUFFER(zeroflash_hcd->out, false);
	INIT_BUFFER(zeroflash_hcd->resp, false);

	zeroflash_hcd->workqueue =
			create_singlethread_workqueue("syna_tcm_zeroflash");
	INIT_WORK(&zeroflash_hcd->config_work,
			zeroflash_download_config_work);
	INIT_DELAYED_WORK(&zeroflash_hcd->romboot_delay_work,
			zeroflash_delayed_romboot_work);

	if (ENABLE_SYS_ZEROFLASH == false)
		goto init_finished;

	zeroflash_hcd->sysfs_dir = kobject_create_and_add(SYSFS_DIR_NAME,
			tcm_hcd->sysfs_dir);
	if (!zeroflash_hcd->sysfs_dir) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to create sysfs directory\n");
		return -EINVAL;
	}

	for (idx = 0; idx < ARRAY_SIZE(attrs); idx++) {
		retval = sysfs_create_file(zeroflash_hcd->sysfs_dir, &(*attrs[idx]).attr);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to create sysfs file\n");
		}
	}

init_finished:
	/* prepare the firmware download process */
	if (tcm_hcd->in_hdl_mode) {
		switch (tcm_hcd->sensor_type) {
		case TYPE_F35:
			zeroflash_do_f35_firmware_download();
			break;
		case TYPE_ROMBOOT:
			tcm_hcd->romboot_download_deferred = true;
			pr_info("SAaiOS_TOUCH_DBG: defer 0x45 until panel enabled fb_ready=%u mode=0x%02x wr_chunk=%u\n",
				tcm_hcd->fb_ready, tcm_hcd->id_info.mode,
				tcm_hcd->wr_chunk_size);
			break;
		default:
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to find valid HDL state (%d)\n", tcm_hcd->sensor_type);
			break;

		}
	}
	return retval;
}

int zeroflash_remove(struct syna_tcm_hcd *tcm_hcd)
{
	int idx;

	if (!zeroflash_hcd)
		goto exit;

	if (zeroflash_hcd->fw_entry)
		release_firmware(zeroflash_hcd->fw_entry);


	if (ENABLE_SYS_ZEROFLASH == true) {

		for (idx = 0; idx < ARRAY_SIZE(attrs); idx++) {
			sysfs_remove_file(zeroflash_hcd->sysfs_dir, &(*attrs[idx]).attr);
		}

		kobject_put(zeroflash_hcd->sysfs_dir);
	}


	cancel_delayed_work_sync(&zeroflash_hcd->romboot_delay_work);
	cancel_work_sync(&zeroflash_hcd->config_work);
	flush_workqueue(zeroflash_hcd->workqueue);
	destroy_workqueue(zeroflash_hcd->workqueue);

	RELEASE_BUFFER(zeroflash_hcd->resp);
	RELEASE_BUFFER(zeroflash_hcd->out);

	kfree(zeroflash_hcd);
	zeroflash_hcd = NULL;

exit:
	complete(&zeroflash_remove_complete);

	return 0;
}

static int zeroflash_syncbox(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	unsigned char *fw_status;

	if (!zeroflash_hcd)
		return 0;

	switch (tcm_hcd->report.id) {
	case REPORT_STATUS:
		fw_status = (unsigned char *)&zeroflash_hcd->fw_status;

		retval = secure_memcpy(fw_status,
				sizeof(zeroflash_hcd->fw_status),
				tcm_hcd->report.buffer.buf,
				tcm_hcd->report.buffer.buf_size,
				sizeof(zeroflash_hcd->fw_status));

		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to copy firmware status\n");
			return retval;
		}
		if (!atomic_read(&tcm_hcd->host_downloading)) {
			static bool skip_logged;

			if (!skip_logged) {
				pr_info("SAaiOS_TOUCH_DBG: skip download_config leftover 0x1b host_downloading=0 mode=0x%02x need_app=%u need_disp=%u need_osh=%u hdl_version=%u cmd=0x%02x\n",
					tcm_hcd->id_info.mode,
					zeroflash_hcd->fw_status.need_app_config,
					zeroflash_hcd->fw_status.need_disp_config,
					zeroflash_hcd->fw_status.need_open_short_config,
					zeroflash_hcd->fw_status.hdl_version,
					tcm_hcd->command);
				skip_logged = true;
			}
			break;
		}
		if (atomic_xchg(&hdl_status_seen, 1))
			break;
		pr_info("SAaiOS_TOUCH_DBG: HDL_OBSERVE 0x1b REPORT_STATUS need_app=%u need_disp=%u need_osh=%u hdl_version=%u invalid_static=%u host_downloading=%d mode=0x%02x (one-shot this 0x45 cycle)\n",
			zeroflash_hcd->fw_status.need_app_config,
			zeroflash_hcd->fw_status.need_disp_config,
			zeroflash_hcd->fw_status.need_open_short_config,
			zeroflash_hcd->fw_status.hdl_version,
			zeroflash_hcd->fw_status.invalid_static_config,
			atomic_read(&tcm_hcd->host_downloading),
			tcm_hcd->id_info.mode);
		zeroflash_download_config();
		break;
	case REPORT_HDL_F35:
		zeroflash_do_f35_firmware_download();
		break;
	case REPORT_HDL_ROMBOOT:
		if (tcm_hcd->romboot_download_deferred || tcm_hcd->fb_ready == 0) {
			tcm_hcd->romboot_download_deferred = true;
			pr_info("SAaiOS_TOUCH_DBG: defer HDL_ROMBOOT 0x45 until panel mode=0x%02x\n",
				tcm_hcd->id_info.mode);
			break;
		}
		zeroflash_do_romboot_firmware_download();
		break;

	default:
		break;
	}

	return 0;
}

static int zeroflash_reinit(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;

	if (!zeroflash_hcd && tcm_hcd->in_hdl_mode) {
		retval = zeroflash_init(tcm_hcd);
		return retval;
	}

	return 0;
}

static struct syna_tcm_module_cb zeroflash_module = {
	.type = TCM_ZEROFLASH,
/*	.init = zeroflash_init, */
	.remove = zeroflash_remove,
	.syncbox = zeroflash_syncbox,
#ifdef REPORT_NOTIFIER
	.asyncbox = NULL,
#endif
	.reinit = zeroflash_reinit,
	.suspend = NULL,
	.resume = NULL,
	.early_suspend = NULL,
};

static int __init zeroflash_module_init(void)
{
	return syna_tcm_add_module(&zeroflash_module, true);
}

static void __exit zeroflash_module_exit(void)
{
	syna_tcm_add_module(&zeroflash_module, false);

	wait_for_completion(&zeroflash_remove_complete);

	return;
}

module_init(zeroflash_module_init);
module_exit(zeroflash_module_exit);

MODULE_AUTHOR("Synaptics, Inc.");
MODULE_DESCRIPTION("Synaptics TCM Zeroflash Module");
MODULE_LICENSE("GPL v2");
