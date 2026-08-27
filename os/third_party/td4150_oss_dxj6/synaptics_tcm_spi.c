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

#include <linux/spi/spi.h>
#include <linux/of_gpio.h>
#include <linux/string.h>
#include <linux/crc32.h>
#include "synaptics_tcm_core.h"

static unsigned char *buf;

static unsigned int buf_size;

static struct spi_transfer *xfer;

static struct syna_tcm_bus_io bus_io;

static struct syna_tcm_hw_interface hw_if;

static struct platform_device *syna_tcm_spi_device;

#ifdef CONFIG_OF
static int parse_dt(struct device *dev, struct syna_tcm_board_data *bdata)
{
	int retval;
	u32 value;
	struct property *prop;
	struct device_node *np = dev->of_node;
	const char *name;
	u32 px_zone[3] = { 0 };
	int lcd_id1_gpio = 0, lcd_id2_gpio = 0, lcd_id3_gpio = 0, dt_lcdtype;
	int fw_name_cnt;
	int lcdtype_cnt;
	int fw_sel_idx = 0;
	int lcdtype = 0;

#if defined(CONFIG_EXYNOS_DPU30)
	int connected;

	connected = get_lcd_info("connected");
	if (connected < 0) {
		input_err(true, dev, "%s: Failed to get lcd info\n", __func__);
		return -EINVAL;
	}

	if (!connected) {
		input_err(true, dev, "%s: lcd is disconnected\n", __func__);
		return -ENODEV;
	}

	input_info(true, dev, "%s: lcd is connected\n", __func__);

	lcdtype = get_lcd_info("id");
	if (lcdtype < 0) {
		input_err(true, dev, "%s: Failed to get lcd info\n", __func__);
		return -EINVAL;
	}
#endif

	fw_name_cnt = of_property_count_strings(np, "synaptics,fw_name");

	if (fw_name_cnt == 0) {
		input_err(true, dev, "%s: no fw_name in DT\n", __func__);
		return -EINVAL;

	} else if (fw_name_cnt == 1) {
		retval = of_property_read_u32(np, "synaptics,lcdtype", &dt_lcdtype);
		if (retval < 0) {
			input_err(true, dev, "%s: failed to read synaptics,lcdtype\n", __func__);

		} else {
			input_info(true, dev, "%s: fw_name_cnt(1), ap lcdtype=0x%06X & dt lcdtype=0x%06X\n",
								__func__, lcdtype, dt_lcdtype);
			if (lcdtype != dt_lcdtype) {
				input_err(true, dev, "%s: panel mismatched, unload driver\n", __func__);
				return -EINVAL;
			}
		}
	} else {

		lcd_id1_gpio = of_get_named_gpio(np, "synaptics,lcdid1-gpio", 0);
		if (gpio_is_valid(lcd_id1_gpio))
			input_info(true, dev, "%s: lcd id1_gpio %d(%d)\n", __func__, lcd_id1_gpio, gpio_get_value(lcd_id1_gpio));
		else {
			input_err(true, dev, "%s: Failed to get synaptics,lcdid1-gpio\n", __func__);
			return -EINVAL;
		}

		lcd_id2_gpio = of_get_named_gpio(np, "synaptics,lcdid2-gpio", 0);
		if (gpio_is_valid(lcd_id2_gpio))
			input_info(true, dev, "%s: lcd id2_gpio %d(%d)\n", __func__, lcd_id2_gpio, gpio_get_value(lcd_id2_gpio));
		else {
			input_err(true, dev, "%s: Failed to get synaptics,lcdid2-gpio\n", __func__);
			return -EINVAL;
		}

		/* support lcd id3 */
		lcd_id3_gpio = of_get_named_gpio(np, "synaptics,lcdid3-gpio", 0);
		if (gpio_is_valid(lcd_id3_gpio)) {
			input_info(true, dev, "%s: lcd id3_gpio %d(%d)\n", __func__, lcd_id3_gpio, gpio_get_value(lcd_id3_gpio));
			fw_sel_idx = (gpio_get_value(lcd_id3_gpio) << 2) | (gpio_get_value(lcd_id2_gpio) << 1) | gpio_get_value(lcd_id1_gpio);

		} else {
			input_err(true, dev, "%s: Failed to get synaptics,lcdid3-gpio and use #1 &#2 id\n", __func__);
			fw_sel_idx = (gpio_get_value(lcd_id2_gpio) << 1) | gpio_get_value(lcd_id1_gpio);
		}

		lcdtype_cnt = of_property_count_u32_elems(np, "synaptics,lcdtype");
		input_info(true, dev, "%s: fw_name_cnt(%d) & lcdtype_cnt(%d) & fw_sel_idx(%d)\n",
					__func__, fw_name_cnt, lcdtype_cnt, fw_sel_idx);

		if (lcdtype_cnt <= 0 || fw_name_cnt <= 0 || lcdtype_cnt <= fw_sel_idx || fw_name_cnt <= fw_sel_idx) {
			input_err(true, dev, "%s: abnormal lcdtype & fw name count, fw_sel_idx(%d)\n", __func__, fw_sel_idx);
			return -EINVAL;
		}
		of_property_read_u32_index(np, "synaptics,lcdtype", fw_sel_idx, &dt_lcdtype);
		input_info(true, dev, "%s: lcd id(%d), ap lcdtype=0x%06X & dt lcdtype=0x%06X\n",
						__func__, fw_sel_idx, lcdtype, dt_lcdtype);

		/* GPIO bits are a BOM hint. AP lcdtype 0x1AF240 matches
		 * neither overlay. Prefer a lcdtype hit, else idx 4
		 * td4150_a12s_boe.bin (0x3A6220).
		 */
		if (lcdtype && dt_lcdtype && lcdtype != dt_lcdtype) {
			int i;
			u32 t = 0;
			const char *n = NULL;
			int picked = -1;

			for (i = 0; i < lcdtype_cnt; i++) {
				of_property_read_u32_index(np, "synaptics,lcdtype", i, &t);
				if (t && t == lcdtype) {
					picked = i;
					break;
				}
			}
			if (picked < 0) {
				for (i = 0; i < fw_name_cnt; i++) {
					n = NULL;
					of_property_read_string_index(np, "synaptics,fw_name", i, &n);
					if (n && strstr(n, "td4150_a12s_boe.bin")) {
						picked = i;
						break;
					}
				}
			}
			if (picked < 0 && fw_name_cnt > 4)
				picked = 4;
			if (picked >= 0) {
				fw_sel_idx = picked;
				of_property_read_u32_index(np, "synaptics,lcdtype",
							fw_sel_idx, &dt_lcdtype);
				pr_info("SAaiOS_TOUCH_DBG: lcdtype mismatch, using idx(%d) dt=0x%06X\n",
					fw_sel_idx, dt_lcdtype);
			}
		}

	}

	of_property_read_string_index(np, "synaptics,fw_name", fw_sel_idx, &bdata->fw_name);
	if (bdata->fw_name == NULL || strlen(bdata->fw_name) == 0) {
		input_err(true, dev, "%s: Failed to get fw name\n", __func__);
		return -EINVAL;
	} else {
		input_info(true, dev, "%s: fw name(%s)\n", __func__, bdata->fw_name);
		pr_info("SAaiOS_TOUCH_DBG: parse_dt fw_name=%s fw_sel_idx=%d ap_lcdtype=0x%06X dt_lcdtype=0x%06X\n",
			bdata->fw_name, fw_sel_idx, lcdtype, dt_lcdtype);
	}

	prop = of_find_property(np, "synaptics,irq-gpio", NULL);
	if (prop && prop->length) {
		bdata->irq_gpio = of_get_named_gpio_flags(np, "synaptics,irq-gpio", 0,
				(enum of_gpio_flags *)&bdata->irq_flags);
	} else {
		bdata->irq_gpio = -1;
	}

	retval = of_property_read_u32(np, "synaptics,irq-on-state", &value);
	if (retval < 0)
		bdata->irq_on_state = 0;
	else
		bdata->irq_on_state = value;

	
	prop = of_find_property(np, "synaptics,cs-gpio", NULL);
	if (prop && prop->length) {
		bdata->cs_gpio = of_get_named_gpio_flags(np, "synaptics,cs-gpio", 0,
				(enum of_gpio_flags *)&bdata->cs_gpio);
	} else {
		bdata->cs_gpio = -1;
	}

	retval = of_property_read_string(np, "synaptics,pwr-reg-name", &name);
	if (retval < 0)
		bdata->pwr_reg_name = NULL;
	else
		bdata->pwr_reg_name = name;

	retval = of_property_read_string(np, "synaptics,bus-reg-name", &name);
	if (retval < 0)
		bdata->bus_reg_name = NULL;
	else
		bdata->bus_reg_name = name;

	prop = of_find_property(np, "synaptics,power-gpio", NULL);
	if (prop && prop->length) {
		bdata->power_gpio = of_get_named_gpio_flags(np, "synaptics,power-gpio", 0, NULL);
	} else {
		bdata->power_gpio = -1;
	}

	prop = of_find_property(np, "synaptics,power-on-state", NULL);
	if (prop && prop->length) {
		retval = of_property_read_u32(np, "synaptics,power-on-state", &value);
		if (retval < 0) {
			input_err(true, dev,
					"Failed to read synaptics,power-on-state property\n");
			return retval;
		} else {
			bdata->power_on_state = value;
		}
	} else {
		bdata->power_on_state = 0;
	}

	prop = of_find_property(np, "synaptics,power-delay-ms", NULL);
	if (prop && prop->length) {
		retval = of_property_read_u32(np, "synaptics,power-delay-ms", &value);
		if (retval < 0) {
			input_err(true, dev,
					"Failed to read synaptics,power-delay-ms property\n");
			return retval;
		} else {
			bdata->power_delay_ms = value;
		}
	} else {
		bdata->power_delay_ms = 0;
	}

	prop = of_find_property(np, "synaptics,reset-gpio", NULL);
	if (prop && prop->length) {
		bdata->reset_gpio = of_get_named_gpio_flags(np,
				"synaptics,reset-gpio", 0, NULL);
	} else {
		bdata->reset_gpio = -1;
	}

	prop = of_find_property(np, "synaptics,reset-on-state", NULL);
	if (prop && prop->length) {
		retval = of_property_read_u32(np, "synaptics,reset-on-state",
				&value);
		if (retval < 0) {
			input_err(true, dev,
					"Failed to read synaptics,reset-on-state property\n");
			return retval;
		} else {
			bdata->reset_on_state = value;
		}
	} else {
		bdata->reset_on_state = 0;
	}

	prop = of_find_property(np, "synaptics,reset-active-ms", NULL);
	if (prop && prop->length) {
		retval = of_property_read_u32(np, "synaptics,reset-active-ms",
				&value);
		if (retval < 0) {
			input_err(true, dev,
					"Failed to read synaptics,reset-active-ms property\n");
			return retval;
		} else {
			bdata->reset_active_ms = value;
		}
	} else {
		bdata->reset_active_ms = 0;
	}

	prop = of_find_property(np, "synaptics,reset-delay-ms", NULL);
	if (prop && prop->length) {
		retval = of_property_read_u32(np, "synaptics,reset-delay-ms",
				&value);
		if (retval < 0) {
			input_err(true, dev,
					"Unable to read synaptics,reset-delay-ms property\n");
			return retval;
		} else {
			bdata->reset_delay_ms = value;
		}
	} else {
		bdata->reset_delay_ms = 0;
	}

	prop = of_find_property(np, "synaptics,tpio-reset-gpio", NULL);
	if (prop && prop->length) {
		bdata->tpio_reset_gpio = of_get_named_gpio_flags(np,
				"synaptics,tpio-reset-gpio", 0, NULL);
	} else {
		bdata->tpio_reset_gpio = -1;
	}

	prop = of_find_property(np, "synaptics,x-flip", NULL);
	bdata->x_flip = prop > 0 ? true : false;

	prop = of_find_property(np, "synaptics,y-flip", NULL);
	bdata->y_flip = prop > 0 ? true : false;

	prop = of_find_property(np, "synaptics,swap-axes", NULL);
	bdata->swap_axes = prop > 0 ? true : false;

	prop = of_find_property(np, "synaptics,byte-delay-us", NULL);
	if (prop && prop->length) {
		retval = of_property_read_u32(np, "synaptics,byte-delay-us", &value);
		if (retval < 0) {
			input_err(true, dev,
					"Unable to read synaptics,byte-delay-us property\n");
			return retval;
		} else {
			bdata->byte_delay_us = value;
		}
	} else {
		bdata->byte_delay_us = 0;
	}

	prop = of_find_property(np, "synaptics,block-delay-us", NULL);
	if (prop && prop->length) {
		retval = of_property_read_u32(np, "synaptics,block-delay-us", &value);
		if (retval < 0) {
			input_err(true, dev,
					"Unable to read synaptics,block-delay-us property\n");
			return retval;
		} else {
			bdata->block_delay_us = value;
		}
	} else {
		bdata->block_delay_us = 0;
	}

	prop = of_find_property(np, "synaptics,spi-mode", NULL);
	if (prop && prop->length) {
		retval = of_property_read_u32(np, "synaptics,spi-mode",
				&value);
		if (retval < 0) {
			input_err(true, dev,
					"Unable to read synaptics,spi-mode property\n");
			return retval;
		} else {
			bdata->spi_mode = value;
		}
	} else {
		bdata->spi_mode = 0;
	}

	prop = of_find_property(np, "synaptics,ubl-max-freq", NULL);
	if (prop && prop->length) {
		retval = of_property_read_u32(np, "synaptics,ubl-max-freq",
				&value);
		if (retval < 0) {
			input_err(true, dev,
					"Unable to read synaptics,ubl-max-freq property\n");
			return retval;
		} else {
			bdata->ubl_max_freq = value;
		}
	} else {
		bdata->ubl_max_freq = 0;
	}

	prop = of_find_property(np, "synaptics,ubl-byte-delay-us", NULL);
	if (prop && prop->length) {
		retval = of_property_read_u32(np, "synaptics,ubl-byte-delay-us",
				&value);
		if (retval < 0) {
			input_err(true, dev,
					"Unable to read synaptics,ubl-byte-delay-us property\n");
			return retval;
		} else {
			bdata->ubl_byte_delay_us = value;
		}
	} else {
		bdata->ubl_byte_delay_us = 0;
	}

	if (of_property_read_string(np, "synaptics,regulator_lcd_vdd", &bdata->regulator_lcd_vdd)) {
		input_err(true, dev, "%s: Failed to get regulator_dvdd name property\n", __func__);
		return -EINVAL;
	}

	if (of_property_read_string(np, "synaptics,regulator_lcd_reset", &bdata->regulator_lcd_reset)) {
		input_err(true, dev, "%s: Failed to get regulator_dvdd name property\n", __func__);
		return -EINVAL;
	}

	if (of_property_read_string(np, "synaptics,regulator_lcd_bl", &bdata->regulator_lcd_bl)) {
		input_err(true, dev, "%s: Failed to get regulator_dvdd name property\n", __func__);
		return -EINVAL;
	}

	bdata->enable_settings_aot = of_property_read_bool(np, "synaptics,enable_settings_aot");

	bdata->pinctrl = devm_pinctrl_get(dev);
	if (IS_ERR(bdata->pinctrl))
		input_err(true, dev, "%s: could not get pinctrl\n", __func__);

	if (of_property_read_u32_array(np, "synaptics,area-size", px_zone, 3)) {
		input_info(true, dev, "Failed to get zone's size\n");
		bdata->area_indicator = 48;
		bdata->area_navigation = 96;
		bdata->area_edge = 60;
	} else {
		bdata->area_indicator = px_zone[0];
		bdata->area_navigation = px_zone[1];
		bdata->area_edge = px_zone[2];
	}
	input_info(true, dev, "%s : zone's size - indicator:%d, navigation:%d, edge:%d\n",
		__func__, bdata->area_indicator, bdata->area_navigation ,bdata->area_edge);

	bdata->support_ear_detect = of_property_read_bool(np, "synaptics,support_ear_detect_mode");
	bdata->prox_lp_scan_enabled = of_property_read_bool(np, "synaptics,prox_lp_scan_enabled");
	input_info(true, dev, "%s: ED:%d, lp scan:%d\n", __func__, bdata->support_ear_detect, bdata->prox_lp_scan_enabled);

	return 0;
}
#endif

static int syna_tcm_spi_alloc_mem(struct syna_tcm_hcd *tcm_hcd,
		unsigned int count, unsigned int size)
{
	static unsigned int xfer_count;
	struct spi_device *spi = to_spi_device(tcm_hcd->pdev->dev.parent);

	if (count > xfer_count) {
		kfree(xfer);
		xfer = kcalloc(count, sizeof(*xfer), GFP_KERNEL);
		if (!xfer) {
			input_err(true, &spi->dev, "Failed to allocate memory for xfer\n");
			xfer_count = 0;
			return -ENOMEM;
		}
		xfer_count = count;
	} else {
		memset(xfer, 0, count * sizeof(*xfer));
	}

	if (size > buf_size) {
		if (buf_size)
			kfree(buf);
		buf = kmalloc(size, GFP_KERNEL);
		if (!buf) {
			input_err(true, &spi->dev, "Failed to allocate memory for buf\n");
			buf_size = 0;
			return -ENOMEM;
		}
		buf_size = size;
	}

	return 0;
}

static int syna_tcm_spi_rmi_read(struct syna_tcm_hcd *tcm_hcd,
		unsigned short addr, unsigned char *data, unsigned int length)
{
	int retval;
	unsigned int idx;
	unsigned int mode;
	unsigned int byte_count;
	struct spi_message msg;
	struct spi_device *spi = to_spi_device(tcm_hcd->pdev->dev.parent);
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	if (tcm_hcd->lp_state == PWR_OFF) {
		input_err(true, tcm_hcd->pdev->dev.parent, "power off in suspend\n");
		return -EIO;
	}

	mutex_lock(&tcm_hcd->io_ctrl_mutex);

	spi_message_init(&msg);

	byte_count = length + 2;

	if (bdata->ubl_byte_delay_us == 0)
		retval = syna_tcm_spi_alloc_mem(tcm_hcd, 2, byte_count);
	else
		retval = syna_tcm_spi_alloc_mem(tcm_hcd, byte_count, 3);
	if (retval < 0) {
		input_err(true, &spi->dev, "Failed to allocate memory\n");
		goto exit;
	}

	buf[0] = (unsigned char)(addr >> 8) | 0x80;
	buf[1] = (unsigned char)addr;

	if (bdata->ubl_byte_delay_us == 0) {
		xfer[0].len = 2;
		xfer[0].tx_buf = buf;
		xfer[0].speed_hz = bdata->ubl_max_freq;
		spi_message_add_tail(&xfer[0], &msg);
		memset(&buf[2], 0xff, length);
		xfer[1].len = length;
		xfer[1].tx_buf = &buf[2];
		xfer[1].rx_buf = data;
		if (bdata->block_delay_us)
			xfer[1].delay_usecs = bdata->block_delay_us;
		xfer[1].speed_hz = bdata->ubl_max_freq;
		spi_message_add_tail(&xfer[1], &msg);
	} else {
		buf[2] = 0xff;
		for (idx = 0; idx < byte_count; idx++) {
			xfer[idx].len = 1;
			if (idx < 2) {
				xfer[idx].tx_buf = &buf[idx];
			} else {
				xfer[idx].tx_buf = &buf[2];
				xfer[idx].rx_buf = &data[idx - 2];
			}
			xfer[idx].delay_usecs = bdata->ubl_byte_delay_us;
			if (bdata->block_delay_us && (idx == byte_count - 1))
				xfer[idx].delay_usecs = bdata->block_delay_us;
			xfer[idx].speed_hz = bdata->ubl_max_freq;
			spi_message_add_tail(&xfer[idx], &msg);
		}
	}

	mode = spi->mode;
	spi->mode = SPI_MODE_3;

	if (bdata->cs_gpio >= 0)
		gpio_set_value(bdata->cs_gpio, 0);

	retval = spi_sync(spi, &msg);
	if (retval == 0) {
		retval = length;
	} else {
		input_err(true, &spi->dev,
				"Failed to complete SPI transfer, error = %d\n", retval);
	}

	if (bdata->cs_gpio >= 0)
		gpio_set_value(bdata->cs_gpio, 1);

	spi->mode = mode;

exit:
	mutex_unlock(&tcm_hcd->io_ctrl_mutex);

	return retval;
}

static int syna_tcm_spi_rmi_write(struct syna_tcm_hcd *tcm_hcd,
		unsigned short addr, unsigned char *data, unsigned int length)
{
	int retval;
	unsigned int mode;
	unsigned int byte_count;
	struct spi_message msg;
	struct spi_device *spi = to_spi_device(tcm_hcd->pdev->dev.parent);
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	if (tcm_hcd->lp_state == PWR_OFF) {
		input_err(true, tcm_hcd->pdev->dev.parent, "power off in suspend\n");
		return -EIO;
	}

	mutex_lock(&tcm_hcd->io_ctrl_mutex);

	spi_message_init(&msg);

	byte_count = length + 2;

	retval = syna_tcm_spi_alloc_mem(tcm_hcd, 1, byte_count);
	if (retval < 0) {
		input_err(true, &spi->dev, "Failed to allocate memory\n");
		goto exit;
	}

	buf[0] = (unsigned char)(addr >> 8) & ~0x80;
	buf[1] = (unsigned char)addr;
	retval = secure_memcpy(&buf[2], buf_size - 2, data, length, length);
	if (retval < 0) {
		input_err(true, &spi->dev, "Failed to copy write data\n");
		goto exit;
	}

	xfer[0].len = byte_count;
	xfer[0].tx_buf = buf;
	if (bdata->block_delay_us)
		xfer[0].delay_usecs = bdata->block_delay_us;
	spi_message_add_tail(&xfer[0], &msg);

	mode = spi->mode;
	spi->mode = SPI_MODE_3;

	if (bdata->cs_gpio >= 0)
		gpio_set_value(bdata->cs_gpio, 0);

	retval = spi_sync(spi, &msg);
	if (retval == 0) {
		retval = length;
	} else {
		input_err(true, &spi->dev,
				"Failed to complete SPI transfer, error = %d\n", retval);
	}
	if (bdata->cs_gpio >= 0)
		gpio_set_value(bdata->cs_gpio, 1);

	spi->mode = mode;

exit:
	mutex_unlock(&tcm_hcd->io_ctrl_mutex);

	return retval;
}

static int syna_tcm_spi_read(struct syna_tcm_hcd *tcm_hcd, unsigned char *data,
		unsigned int length)
{
	int retval;
	unsigned int idx;
	struct spi_message msg;
	struct spi_device *spi = to_spi_device(tcm_hcd->pdev->dev.parent);
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	if (tcm_hcd->lp_state == PWR_OFF) {
		input_err(true, tcm_hcd->pdev->dev.parent, "power off in suspend\n");
		return -EIO;
	}

	mutex_lock(&tcm_hcd->io_ctrl_mutex);

	spi_message_init(&msg);

	if (bdata->byte_delay_us == 0)
		retval = syna_tcm_spi_alloc_mem(tcm_hcd, 1, length);
	else
		retval = syna_tcm_spi_alloc_mem(tcm_hcd, length, 1);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to allocate memory\n");
		goto exit;
	}

	if (bdata->byte_delay_us == 0) {
		memset(buf, 0xff, length);
		xfer[0].len = length;
		xfer[0].tx_buf = buf;
		xfer[0].rx_buf = data;
		if (bdata->block_delay_us)
			xfer[0].delay_usecs = bdata->block_delay_us;
		spi_message_add_tail(&xfer[0], &msg);
	} else {
		buf[0] = 0xff;
		for (idx = 0; idx < length; idx++) {
			xfer[idx].len = 1;
			xfer[idx].tx_buf = buf;
			xfer[idx].rx_buf = &data[idx];
			xfer[idx].delay_usecs = bdata->byte_delay_us;
			if (bdata->block_delay_us && (idx == length - 1))
				xfer[idx].delay_usecs = bdata->block_delay_us;
			spi_message_add_tail(&xfer[idx], &msg);
		}
	}

	if (bdata->cs_gpio >= 0)
		gpio_set_value(bdata->cs_gpio, 0);

	retval = spi_sync(spi, &msg);
	if (retval == 0) {
		retval = length;
	} else {
		input_err(true, &spi->dev,
				"Failed to complete SPI transfer, error = %d\n", retval);
	}

	if (bdata->cs_gpio >= 0)
		gpio_set_value(bdata->cs_gpio, 1);
exit:
	mutex_unlock(&tcm_hcd->io_ctrl_mutex);

	return retval;
}

/* since63: dump the actual SPI TX buffer for one-shot 0x45 immediately
 * before spi_sync. write_message layout:
 *   tx[0]=cmd 0x45, tx[1..2]=u16 payload_len LE, tx[3..16]=reserved[14],
 *   tx[17..]=APP_CODE (97280). SPI len = 3 + 14 + 97280 = 97297.
 * syna_tcm_spi_write with byte_delay_us==0 uses data as xfer[0].tx_buf.
 */
#define SAAIOS_0X45_SPI_LEN		97297U
#define SAAIOS_0X45_APP_SIZE		97280U
#define SAAIOS_0X45_TCM_HDR		3U
#define SAAIOS_0X45_RESERVED		14U
#define SAAIOS_0X45_APP_OFF		(SAAIOS_0X45_TCM_HDR + SAAIOS_0X45_RESERVED)

static void syna_hex_n(char *out, size_t outlen, const unsigned char *p,
		unsigned int n)
{
	unsigned int i, pos = 0;

	if (!out || !outlen)
		return;
	out[0] = '\0';
	if (!p)
		return;
	for (i = 0; i < n && pos + 3 < outlen; i++)
		pos += scnprintf(out + pos, outlen - pos, "%02x ", p[i]);
}

static void syna_dump_0x45_spi_tx(const unsigned char *tx, unsigned int len,
		unsigned int byte_delay_us, const void *xfer_tx)
{
	char hex[100];
	unsigned int i, nz = 0, app_off = SAAIOS_0X45_APP_OFF;
	unsigned int app_sz = SAAIOS_0X45_APP_SIZE;
	unsigned int mid;
	int first_nz = -1;
	u32 spi_crc, app_crc;

	if (!tx || len < app_off + app_sz)
		return;

	pr_info("SAaiOS_TOUCH_DBG: 0x45 SPI TX dump immediately before spi_sync len=%u tx=%p xfer0_tx=%p same=%d byte_delay_us=%u (oneshot xfer[0].tx_buf=data)\n",
		len, tx, xfer_tx, (xfer_tx == (const void *)tx) ? 1 : 0,
		byte_delay_us);
	pr_info("SAaiOS_TOUCH_DBG: SPI offsets: tcm_hdr=tx[0..2] (cmd + u16 payload_len LE); reserved=tx[3..16] (14B); APP_CODE=tx[%u..%u] size=%u; SPI last=tx[%u] (APP_CODE last == SPI last, no trailer)\n",
		app_off, app_off + app_sz - 1, app_sz, len - 1);

	syna_hex_n(hex, sizeof(hex), tx, 32);
	pr_info("SAaiOS_TOUCH_DBG: SPI tx[0..31]=%s\n", hex);
	pr_info("SAaiOS_TOUCH_DBG: cmd=tx[0]=0x%02x (expect 0x45) len_le=tx[1..2]=%02x %02x payload_u16=%u\n",
		tx[0], tx[1], tx[2],
		(unsigned int)tx[1] | ((unsigned int)tx[2] << 8));

	syna_hex_n(hex, sizeof(hex), tx + 3, 14);
	pr_info("SAaiOS_TOUCH_DBG: reserved tx[3..16]=%s (expect 01 00..00)\n", hex);

	syna_hex_n(hex, sizeof(hex), tx + app_off, 16);
	pr_info("SAaiOS_TOUCH_DBG: APP_CODE start tx[%u..]=%s\n",
		app_off, hex);

	mid = app_off + (app_sz / 2);
	syna_hex_n(hex, sizeof(hex), tx + mid, 16);
	pr_info("SAaiOS_TOUCH_DBG: APP_CODE mid tx[%u..]=%s\n",
		mid, hex);

	syna_hex_n(hex, sizeof(hex), tx + app_off + app_sz - 16, 16);
	pr_info("SAaiOS_TOUCH_DBG: APP_CODE last 16 tx[%u..%u]=%s\n",
		app_off + app_sz - 16, app_off + app_sz - 1, hex);

	syna_hex_n(hex, sizeof(hex), tx + len - 16, 16);
	pr_info("SAaiOS_TOUCH_DBG: SPI buf last 16 tx[%u..%u]=%s (same as APP_CODE last 16)\n",
		len - 16, len - 1, hex);

	for (i = 0; i < app_sz; i++) {
		if (tx[app_off + i]) {
			if (first_nz < 0)
				first_nz = (int)i;
			nz++;
		}
	}
	pr_info("SAaiOS_TOUCH_DBG: APP_CODE nonzero=%u first_nz_off=%d APP_CODE[0]=0x%02x APP_CODE[%u]=0x%02x (good APP_CODE, SAAIOS_FORCE_CORRUPT_APP=0)\n",
		nz, first_nz, tx[app_off], app_sz - 1, tx[app_off + app_sz - 1]);

	spi_crc = crc32(~0, tx, len) ^ ~0;
	app_crc = crc32(~0, tx + app_off, app_sz) ^ ~0;
	pr_info("SAaiOS_TOUCH_DBG: CRC32 entire SPI TX buf=0x%08x (len=%u) CRC32 APP_CODE region=0x%08x (tx[%u] size=%u)\n",
		spi_crc, len, app_crc, app_off, app_sz);
}

/* since76: dump experiment SPI TX (0x05 / 0x24 / 0x30) with seq. Expect
 * 0x05: 05 01 00 11 actual_length=4. 0x24: 24 03 00 01 01 00.
 * 0x30 oneshot: 30 02 10 02 01 … prepared_len≈4100+ (not 512).
 */
static void syna_dump_exp_spi_prepared(const unsigned char *tx, unsigned int len)
{
	unsigned char b0, b1, b2, b3, b4, b5;

	if (!tx || !len)
		return;
	if (tx[0] != CMD_ENABLE_REPORT && tx[0] != CMD_SET_DYNAMIC_CONFIG &&
			tx[0] != CMD_DOWNLOAD_CONFIG)
		return;

	b0 = tx[0];
	b1 = len > 1 ? tx[1] : 0;
	b2 = len > 2 ? tx[2] : 0;
	b3 = len > 3 ? tx[3] : 0;
	b4 = len > 4 ? tx[4] : 0;
	b5 = len > 5 ? tx[5] : 0;
	pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x%02x TX first=%02x %02x %02x %02x %02x %02x prepared_len=%u\n",
		syna_tcm_saaios_exp_seq(), b0, b0, b1, b2, b3, b4, b5, len);
}

static void syna_dump_exp_spi_done(unsigned char cmd, int spi_sync_retval,
		unsigned int actual_length, unsigned int prepared_len)
{
	if (cmd != CMD_ENABLE_REPORT && cmd != CMD_SET_DYNAMIC_CONFIG &&
			cmd != CMD_DOWNLOAD_CONFIG)
		return;
	pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x%02x spi_sync retval=%d actual_length=%u prepared_len=%u\n",
		syna_tcm_saaios_exp_seq(), cmd, spi_sync_retval, actual_length,
		prepared_len);
}

static int syna_tcm_spi_write(struct syna_tcm_hcd *tcm_hcd, unsigned char *data,
		unsigned int length)
{
	int retval;
	unsigned int idx;
	struct spi_message msg;
	struct spi_device *spi = to_spi_device(tcm_hcd->pdev->dev.parent);
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	if (tcm_hcd->lp_state == PWR_OFF) {
		input_err(true, tcm_hcd->pdev->dev.parent, "power off in suspend\n");
		return -EIO;
	}

	mutex_lock(&tcm_hcd->io_ctrl_mutex);

	spi_message_init(&msg);

	if (bdata->byte_delay_us == 0)
		retval = syna_tcm_spi_alloc_mem(tcm_hcd, 1, 0);
	else
		retval = syna_tcm_spi_alloc_mem(tcm_hcd, length, 0);
	if (retval < 0) {
		input_err(true, &spi->dev, "Failed to allocate memory\n");
		goto exit;
	}

	if (bdata->byte_delay_us == 0) {
		xfer[0].len = length;
		xfer[0].tx_buf = data;
		if (bdata->block_delay_us)
			xfer[0].delay_usecs = bdata->block_delay_us;
		spi_message_add_tail(&xfer[0], &msg);
	} else {
		for (idx = 0; idx < length; idx++) {
			xfer[idx].len = 1;
			xfer[idx].tx_buf = &data[idx];
			xfer[idx].delay_usecs = bdata->byte_delay_us;
			if (bdata->block_delay_us && (idx == length - 1))
				xfer[idx].delay_usecs = bdata->block_delay_us;
			spi_message_add_tail(&xfer[idx], &msg);
		}
	}

	if (bdata->cs_gpio >= 0)
		gpio_set_value(bdata->cs_gpio, 0);

	if (data && length >= SAAIOS_0X45_SPI_LEN && data[0] == CMD_ROMBOOT_DOWNLOAD)
		syna_dump_0x45_spi_tx(data, length, bdata->byte_delay_us,
			(bdata->byte_delay_us == 0) ? xfer[0].tx_buf : NULL);

	if (data && length)
		syna_dump_exp_spi_prepared(data, length);

	retval = spi_sync(spi, &msg);
	if (data && length)
		syna_dump_exp_spi_done(data[0], retval, msg.actual_length, length);
	if (retval == 0) {
		retval = length;
	} else {
		input_err(true, &spi->dev,
				"Failed to complete SPI transfer, error = %d\n", retval);
	}

	if (bdata->cs_gpio >= 0)
		gpio_set_value(bdata->cs_gpio, 1);

exit:
	mutex_unlock(&tcm_hcd->io_ctrl_mutex);

	return retval;
}

static int syna_tcm_spi_probe(struct spi_device *spi)
{
	int retval;

#ifdef CONFIG_OF
	hw_if.bdata = devm_kzalloc(&spi->dev, sizeof(*hw_if.bdata), GFP_KERNEL);
	if (!hw_if.bdata) {
		input_err(true, &spi->dev, "Failed to allocate memory for board data\n");
		return -ENOMEM;
	}
	retval = parse_dt(&spi->dev, hw_if.bdata);
	if (retval < 0) {
		input_err(true, &spi->dev, "%s : parse_dt failed\n", __func__);
		return -EINVAL;
	}
#else
	hw_if.bdata = spi->dev.platform_data;
#endif

	if (spi->master->flags & SPI_MASTER_HALF_DUPLEX) {
		input_err(true, &spi->dev, "Full duplex not supported by host\n");
		return -EIO;
	}

	syna_tcm_spi_device = platform_device_alloc(PLATFORM_DRIVER_NAME, 0);
	if (!syna_tcm_spi_device) {
		input_err(true, &spi->dev, "Failed to allocate platform device\n");
		return -ENOMEM;
	}

	switch (hw_if.bdata->spi_mode) {
	case 0:
		spi->mode = SPI_MODE_0;
		break;
	case 1:
		spi->mode = SPI_MODE_1;
		break;
	case 2:
		spi->mode = SPI_MODE_2;
		break;
	case 3:
		spi->mode = SPI_MODE_3;
		break;
	}

	bus_io.type = BUS_SPI;
	bus_io.read = syna_tcm_spi_read;
	bus_io.write = syna_tcm_spi_write;
	bus_io.rmi_read = syna_tcm_spi_rmi_read;
	bus_io.rmi_write = syna_tcm_spi_rmi_write;

	hw_if.bus_io = &bus_io;

	spi->bits_per_word = 8;

	retval = spi_setup(spi);
	if (retval < 0) {
		input_err(true, &spi->dev, "Failed to set up SPI protocol driver\n");
		return retval;
	}

	/* Stock a12s DTBO: spi-max-frequency 7 MHz, synaptics,spi-mode 3.
	 * Do not unbind synaptics_tcm_spi. reset-gpio is -1; never pulse
	 * gpio_lcd_rst.
	 */
	if (spi->max_speed_hz > 7000000) {
		pr_info("SAaiOS_TOUCH_DBG: SPI %u Hz faster than stock 7MHz, clamping\n",
			spi->max_speed_hz);
		spi->max_speed_hz = 7000000;
		retval = spi_setup(spi);
		if (retval < 0) {
			input_err(true, &spi->dev, "Failed to clamp SPI frequency\n");
			return retval;
		}
	}
	pr_info("SAaiOS_TOUCH_DBG: SPI speed=%u Hz mode=%u ubl_max_freq=%u spi_mode_dt=%u\n",
		spi->max_speed_hz, spi->mode,
		hw_if.bdata->ubl_max_freq, hw_if.bdata->spi_mode);

	syna_tcm_spi_device->dev.parent = &spi->dev;
	syna_tcm_spi_device->dev.platform_data = &hw_if;

	retval = platform_device_add(syna_tcm_spi_device);
	if (retval < 0) {
		input_err(true, &spi->dev, "Failed to add platform device\n");
		return retval;
	}

	return 0;
}

static int syna_tcm_spi_remove(struct spi_device *spi)
{
	syna_tcm_spi_device->dev.platform_data = NULL;

	platform_device_unregister(syna_tcm_spi_device);

	return 0;
}

static const struct spi_device_id syna_tcm_id_table[] = {
	{SPI_MODULE_NAME, 0},
	{},
};
MODULE_DEVICE_TABLE(spi, syna_tcm_id_table);

#ifdef CONFIG_OF
static struct of_device_id syna_tcm_of_match_table[] = {
	{
		.compatible = "synaptics,tcm-spi",
	},
	{},
};
MODULE_DEVICE_TABLE(of, syna_tcm_of_match_table);
#else
#define syna_tcm_of_match_table NULL
#endif

static struct spi_driver syna_tcm_spi_driver = {
	.driver = {
		.name = SPI_MODULE_NAME,
		.owner = THIS_MODULE,
		.of_match_table = syna_tcm_of_match_table,
	},
	.probe = syna_tcm_spi_probe,
	.remove = syna_tcm_spi_remove,
	.id_table = syna_tcm_id_table,
};

int syna_tcm_bus_init(void)
{
	return spi_register_driver(&syna_tcm_spi_driver);
}
EXPORT_SYMBOL(syna_tcm_bus_init);

void syna_tcm_bus_exit(void)
{
	kfree(buf);

	kfree(xfer);

	spi_unregister_driver(&syna_tcm_spi_driver);

	return;
}
EXPORT_SYMBOL(syna_tcm_bus_exit);

MODULE_AUTHOR("Synaptics, Inc.");
MODULE_DESCRIPTION("Synaptics TCM SPI Bus Module");
MODULE_LICENSE("GPL v2");
