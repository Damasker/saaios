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
#include <linux/kthread.h>
#include <linux/interrupt.h>
#include <linux/kobject.h>
#include <linux/sysfs.h>
#include <linux/string.h>
#include <linux/regulator/consumer.h>
#include "synaptics_tcm_core.h"

/* #define RESET_ON_RESUME */

/* #define RESUME_EARLY_UNBLANK */

#define RESET_ON_RESUME_DELAY_MS 50

#define PREDICTIVE_READING 

#define MIN_READ_LENGTH 9

#ifndef SAAIOS_TRC_READ_LEN
#define SAAIOS_TRC_READ_LEN 133
#endif

/* #define FORCE_RUN_APPLICATION_FIRMWARE */

#define NOTIFIER_PRIORITY 2

#ifdef CONFIG_SEC_FACTORY
#define RESPONSE_TIMEOUT_MS 3000
#else
#define RESPONSE_TIMEOUT_MS 1000
#endif

#define APP_STATUS_POLL_TIMEOUT_MS 1000

#define APP_STATUS_POLL_MS 100

#define ENABLE_IRQ_DELAY_MS 20

#define FALL_BACK_ON_POLLING

#define POLLING_DELAY_MS 5

#define MODE_SWITCH_DELAY_MS 100

#define READ_RETRY_US_MIN 5000

#define READ_RETRY_US_MAX 10000

#define WRITE_DELAY_US_MIN 500

#define WRITE_DELAY_US_MAX 1000

#define ROMBOOT_DOWNLOAD_UNIT 16

#define PDT_END_ADDR 0x00ee

#define RMI_UBL_FN_NUMBER 0x35

static struct syna_tcm_module_pool mod_pool;
bool shutdown_is_on_going_tsp;

DECLARE_COMPLETION(response_complete);

static unsigned int since58_irq_cnt;
static unsigned int since58_rx_cnt;
static bool since58_sent_0x45;
static unsigned int saaios_0x45_chunk_idx;
static unsigned int saaios_0x45_chunks;

#define SAAIOS_IRQ_DETAIL_MAX		8
#define SAAIOS_IRQ_DRAIN_MAX		32
#define SAAIOS_LEFTOVER_LOG_N		8
#define SAAIOS_LEFTOVER_LOG_EVERY	1000

static unsigned int saaios_irq_detail_n;
static unsigned int saaios_leftover_1b_n;
static int saaios_last_0x45_retval = 0x7fffffff; /* never returned */
static atomic_t saaios_irq_held = ATOMIC_INIT(0);
static struct syna_tcm_hcd *saaios_storm_tcm;
static void saaios_irq_storm_recovery(struct work_struct *work);
static DECLARE_WORK(saaios_irq_storm_work, saaios_irq_storm_recovery);

static bool saaios_irq_hold_blocked(struct syna_tcm_hcd *tcm_hcd)
{
	if (!tcm_hcd)
		return true;
	if (atomic_read(&tcm_hcd->host_downloading))
		return true;
	if (tcm_hcd->command != CMD_NONE)
		return true; /* includes 0x20 / 0x25 / 0x05; do not hold during REINIT */
	if (tcm_hcd->command == CMD_DOWNLOAD_CONFIG)
		return true;
	if (atomic_read(&tcm_hcd->command_status) == CMD_BUSY)
		return true;
	if (since58_sent_0x45 && saaios_last_0x45_retval == 0x7fffffff)
		return true;
	return false;
}

static void saaios_irq_emergency_hold(struct syna_tcm_hcd *tcm_hcd,
		const char *why)
{
	if (!tcm_hcd)
		return;
	if (saaios_irq_hold_blocked(tcm_hcd)) {
		static unsigned int skip_n;

		skip_n++;
		if (skip_n <= 2 || (skip_n % SAAIOS_LEFTOVER_LOG_EVERY) == 0)
			pr_info("SAaiOS_TOUCH_DBG: IRQ emergency hold skipped (0x45/cmd still pending) n=%u why='%s' irq_cnt=%u leftover_1b=%u sent_0x45=%d last_0x45_retval=%d wr_chunk=%u cmd=0x%02x host_downloading=%d mode=0x%02x cmd_status=%d\n",
				skip_n, why ? why : "-", since58_irq_cnt,
				saaios_leftover_1b_n,
				since58_sent_0x45 ? 1 : 0, saaios_last_0x45_retval,
				tcm_hcd->wr_chunk_size, tcm_hcd->command,
				atomic_read(&tcm_hcd->host_downloading),
				tcm_hcd->id_info.mode,
				atomic_read(&tcm_hcd->command_status));
		return;
	}
	if (atomic_cmpxchg(&saaios_irq_held, 0, 1) != 0)
		return;
	if (tcm_hcd->irq_enabled)
		disable_irq_nosync(tcm_hcd->irq);
	tcm_hcd->irq_enabled = false;
	pr_info("SAaiOS_TOUCH_DBG: IRQ emergency hold why='%s' irq_cnt=%u leftover_1b=%u sent_0x45=%d last_0x45_retval=%d wr_chunk=%u cmd=0x%02x host_downloading=%d mode=0x%02x\n",
		why ? why : "-", since58_irq_cnt, saaios_leftover_1b_n,
		since58_sent_0x45 ? 1 : 0, saaios_last_0x45_retval,
		tcm_hcd->wr_chunk_size, tcm_hcd->command,
		atomic_read(&tcm_hcd->host_downloading),
		tcm_hcd->id_info.mode);
	if (tcm_hcd->helper.workqueue)
		queue_work(tcm_hcd->helper.workqueue, &saaios_irq_storm_work);
}

static void saaios_irq_storm_recovery(struct work_struct *work)
{
	struct syna_tcm_hcd *tcm_hcd = saaios_storm_tcm;
	int gpio = -1;

	if (!tcm_hcd)
		return;
	if (tcm_hcd->hw_if && tcm_hcd->hw_if->bdata &&
			tcm_hcd->hw_if->bdata->irq_gpio >= 0)
		gpio = gpio_get_value(tcm_hcd->hw_if->bdata->irq_gpio);
	pr_info("SAaiOS_TOUCH_DBG: IRQ storm recovery (IRQ held, no opcode) gpio=%d irq_en=%d sent_0x45=%d last_0x45_retval=%d wr_chunk=%u cmd=0x%02x host_downloading=%d mode=0x%02x leftover_1b=%u irq_cnt=%u rx_cnt=%u (0x7fffffff=0x45 never returned)\n",
		gpio, tcm_hcd->irq_enabled ? 1 : 0,
		since58_sent_0x45 ? 1 : 0, saaios_last_0x45_retval,
		tcm_hcd->wr_chunk_size, tcm_hcd->command,
		atomic_read(&tcm_hcd->host_downloading),
		tcm_hcd->id_info.mode, saaios_leftover_1b_n,
		since58_irq_cnt, since58_rx_cnt);
}

void syna_tcm_saaios_reset_hdl_observe(void)
{
	saaios_irq_detail_n = 0;
	saaios_leftover_1b_n = 0;
	atomic_set(&saaios_irq_held, 0);
}

static atomic_t saaios_allow_hdl_reinit = ATOMIC_INIT(0);

void syna_tcm_saaios_allow_hdl_reinit(int allow)
{
	atomic_set(&saaios_allow_hdl_reinit, allow ? 1 : 0);
}

void syna_tcm_dump_identify(struct syna_tcm_hcd *tcm_hcd,
		const unsigned char *p, unsigned int len, const char *when)
{
	unsigned char part[17];
	unsigned int packrat = 0, max_write = 0;
	unsigned char b0 = 0, b1 = 0, b2 = 0, b3 = 0;
	unsigned char ver = 0, mode = 0;
	unsigned int n;

	memset(part, 0, sizeof(part));
	if (!p && tcm_hcd) {
		p = (const unsigned char *)&tcm_hcd->id_info;
		len = sizeof(tcm_hcd->id_info);
	}
	if (p && len >= 1)
		ver = p[0];
	if (p && len >= 2)
		mode = p[1];
	if (p && len > 2) {
		n = len - 2;
		if (n > 16)
			n = 16;
		memcpy(part, p + 2, n);
	}
	if (p && len >= 22) {
		b0 = p[18];
		b1 = p[19];
		b2 = p[20];
		b3 = p[21];
		packrat = le4_to_uint(p + 18);
	}
	if (p && len >= 24)
		max_write = le2_to_uint(p + 22);

	pr_info("SAaiOS_TOUCH_DBG: IDENTIFY %s payload_len=%u ver=0x%02x mode=0x%02x part='%s' build=%02x %02x %02x %02x packrat=%u max_write=%u\n",
		when ? when : "-", len, ver, mode, part, b0, b1, b2, b3, packrat, max_write);
}

unsigned int syna_tcm_saaios_irq_cnt(void)
{
	return since58_irq_cnt;
}

unsigned int syna_tcm_saaios_rx_cnt(void)
{
	return since58_rx_cnt;
}

static unsigned int saaios_report_touch_n;
static unsigned int saaios_exp_seq;
static int saaios_last_cmd;
static int saaios_last_retval;
static unsigned char saaios_last_response;
static int saaios_live20;
static unsigned int saaios_pending_cmd;
static unsigned int saaios_pending_delay_ms;
static atomic_t saaios_exp_busy = ATOMIC_INIT(0);
static struct syna_tcm_hcd *saaios_touch_tcm;
static struct workqueue_struct *saaios_touch_wq;
static struct work_struct saaios_touch_work;
static struct kobject *saaios_touch_kobj;
static int saaios_reinit_ok;
static int saaios_reinit_fw_mode;
static ktime_t saaios_reinit_ok_kt;
static unsigned int saaios_ladder_i;
static int saaios_ladder_active;
static struct delayed_work saaios_ladder_dwork;

enum saaios_exp_state {
	SAAIOS_ST_READY = 0,
	SAAIOS_ST_BUSY,
	SAAIOS_ST_DEAD,
};

static int saaios_state = SAAIOS_ST_READY;

/* Auto live20 (empty GET 0x20) after REINIT unlock (no global read-floor).
 * Ladder starts AFTER mutex_unlock(&reset_mutex): delay=0 under the mutex
 * raced the successful REINIT 0x20 and timed out, aborting 10/100/500.
 * Do NOT auto-send 0x05/0x24/0x30.
 */
static const unsigned int saaios_live20_delays_ms[] = { 10, 100, 500, 1000 };
#define SAAIOS_LIVE20_LADDER_N ARRAY_SIZE(saaios_live20_delays_ms)

unsigned int syna_tcm_saaios_exp_seq(void)
{
	return saaios_exp_seq;
}

static const char *saaios_state_str(void)
{
	if (saaios_state == SAAIOS_ST_DEAD)
		return "dead";
	if (saaios_state == SAAIOS_ST_BUSY)
		return "busy";
	return "ready";
}

static int saaios_attn_gpio(struct syna_tcm_hcd *tcm_hcd)
{
	if (!tcm_hcd || !tcm_hcd->hw_if || !tcm_hcd->hw_if->bdata)
		return -1;
	if (tcm_hcd->hw_if->bdata->irq_gpio < 0)
		return -1;
	return gpio_get_value(tcm_hcd->hw_if->bdata->irq_gpio);
}

static int saaios_token_is(const char *p, const char *tok)
{
	size_t n = strlen(tok);

	if (strncmp(p, tok, n))
		return 0;
	return p[n] == 0 || p[n] == '\n' || p[n] == '\r' || p[n] == ' ' ||
		p[n] == '\t';
}

static void saaios_mark_dead(int retval, const char *why)
{
	saaios_last_retval = retval;
	saaios_last_response = 0xff;
	saaios_state = SAAIOS_ST_DEAD;
	saaios_ladder_active = 0;
	pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: state=dead retval=%d response=ff (%s; block until reboot)\n",
		saaios_exp_seq, retval, why ? why : "-");
}

static void saaios_pre_tx_log(struct syna_tcm_hcd *tcm_hcd, unsigned int cmd,
		unsigned int delay_ms)
{
	pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: pre-TX cmd=0x%02x delay_ms=%u irq_cnt=%u attn=%d irq_en=%d mode=0x%02x hdl=%d lp_state=%u wr_chunk=%u\n",
		saaios_exp_seq, cmd, delay_ms, since58_irq_cnt,
		saaios_attn_gpio(tcm_hcd), tcm_hcd->irq_enabled ? 1 : 0,
		tcm_hcd->id_info.mode,
		atomic_read(&tcm_hcd->host_downloading),
		tcm_hcd->lp_state, tcm_hcd->wr_chunk_size);
}

static int saaios_live_0x20(struct syna_tcm_hcd *tcm_hcd)
{
	int retval20;

	if (!tcm_hcd || !tcm_hcd->identify)
		return -ENODEV;
	retval20 = tcm_hcd->identify(tcm_hcd, false);
	pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: live 0x20 retval=%d app_status=0x%04x mode=0x%02x ATTN=%d irq=%u\n",
		saaios_exp_seq, retval20, tcm_hcd->app_status,
		tcm_hcd->id_info.mode, saaios_attn_gpio(tcm_hcd),
		since58_irq_cnt);
	return retval20;
}

static bool saaios_exp_guards_ok(struct syna_tcm_hcd *tcm_hcd)
{
	return since58_sent_0x45 && saaios_last_0x45_retval == 0 &&
		saaios_reinit_ok &&
		IS_FW_MODE(tcm_hcd->id_info.mode) &&
		!atomic_read(&tcm_hcd->host_downloading);
}

static void saaios_ladder_kick(void);

static void saaios_touch_work_fn(struct work_struct *work)
{
	struct syna_tcm_hcd *tcm_hcd = saaios_touch_tcm;
	unsigned char *resp_buf = NULL;
	unsigned int resp_size = 0;
	unsigned int resp_len = 0;
	unsigned char response_code = 0;
	unsigned char report_id = REPORT_TOUCH;
	unsigned int pending = saaios_pending_cmd;
	unsigned int delay_ms = saaios_pending_delay_ms;
	int retval = 0;
	int live20 = 0;
	int from_ladder;

	(void)work;

	if (!tcm_hcd) {
		saaios_state = SAAIOS_ST_READY;
		atomic_set(&saaios_exp_busy, 0);
		return;
	}

	from_ladder = saaios_ladder_active &&
		pending == CMD_GET_APPLICATION_INFO;

	if (!saaios_exp_guards_ok(tcm_hcd)) {
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: skip guards sent_0x45=%d 0x45_retval=%d reinit_ok=%d mode=0x%02x hdl=%d\n",
			saaios_exp_seq, since58_sent_0x45 ? 1 : 0,
			saaios_last_0x45_retval, saaios_reinit_ok,
			tcm_hcd->id_info.mode,
			atomic_read(&tcm_hcd->host_downloading));
		saaios_last_cmd = pending;
		saaios_mark_dead(-EAGAIN, "guards");
		atomic_set(&saaios_exp_busy, 0);
		return;
	}

	saaios_last_cmd = pending;
	saaios_pre_tx_log(tcm_hcd, pending, delay_ms);

	switch (pending) {
	case CMD_ENABLE_REPORT:
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x05 TX payload=0x%02x mode=0x%02x wr_chunk=%u ATTN=%d irq=%u\n",
			saaios_exp_seq, report_id, tcm_hcd->id_info.mode,
			tcm_hcd->wr_chunk_size, saaios_attn_gpio(tcm_hcd),
			since58_irq_cnt);
		retval = tcm_hcd->write_message(tcm_hcd, CMD_ENABLE_REPORT,
				&report_id, 1, &resp_buf, &resp_size, &resp_len,
				&response_code, 0);
		saaios_last_retval = retval;
		saaios_last_response = (retval < 0) ? 0xff : response_code;
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: done 0x05 retval=%d response=%02x resp_len=%u ATTN=%d irq=%u\n",
			saaios_exp_seq, retval, saaios_last_response, resp_len,
			saaios_attn_gpio(tcm_hcd), since58_irq_cnt);
		kfree(resp_buf);
		break;
	case CMD_DOWNLOAD_CONFIG:
		/* Optional sysfs only — not in post-boot menu. Late 0x30 LIVE -62. */
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x30 TX stock zeroflash_download_app_config oneshot wr_chunk=0 ATTN=%d irq=%u\n",
			saaios_exp_seq, saaios_attn_gpio(tcm_hcd),
			since58_irq_cnt);
		retval = zeroflash_saaios_download_app_config();
		saaios_last_retval = retval;
		saaios_last_response = (retval < 0) ? 0xff :
			tcm_hcd->response_code;
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: done 0x30 retval=%d response=%02x ATTN=%d irq=%u\n",
			saaios_exp_seq, retval, saaios_last_response,
			saaios_attn_gpio(tcm_hcd), since58_irq_cnt);
		break;
	case CMD_SET_DYNAMIC_CONFIG:
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x24 TX DC_NO_DOZE=1 payload=01 01 00 ATTN=%d irq=%u\n",
			saaios_exp_seq, saaios_attn_gpio(tcm_hcd),
			since58_irq_cnt);
		if (!tcm_hcd->set_dynamic_config)
			retval = -ENODEV;
		else
			retval = tcm_hcd->set_dynamic_config(tcm_hcd,
					DC_NO_DOZE, 1);
		saaios_last_retval = retval;
		saaios_last_response = (retval < 0) ? 0xff :
			tcm_hcd->response_code;
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: done 0x24 retval=%d response=%02x ATTN=%d irq=%u\n",
			saaios_exp_seq, retval, saaios_last_response,
			saaios_attn_gpio(tcm_hcd), since58_irq_cnt);
		break;
	case CMD_RUN_APPLICATION_FIRMWARE:
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x14 RUN_APPLICATION_FIRMWARE empty payload mode=0x%02x packrat=%u part='%s' ATTN=%d irq=%u\n",
			saaios_exp_seq, tcm_hcd->id_info.mode,
			tcm_hcd->packrat_number, tcm_hcd->id_info.part_number,
			saaios_attn_gpio(tcm_hcd), since58_irq_cnt);
		retval = tcm_hcd->write_message(tcm_hcd,
				CMD_RUN_APPLICATION_FIRMWARE, NULL, 0,
				&resp_buf, &resp_size, &resp_len,
				&response_code, MODE_SWITCH_DELAY_MS);
		saaios_last_retval = retval;
		saaios_last_response = (retval < 0) ? 0xff : response_code;
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: done 0x14 retval=%d response=%02x resp_len=%u mode=0x%02x packrat=%u part='%s' ATTN=%d irq=%u\n",
			saaios_exp_seq, retval, saaios_last_response, resp_len,
			tcm_hcd->id_info.mode, tcm_hcd->packrat_number,
			tcm_hcd->id_info.part_number,
			saaios_attn_gpio(tcm_hcd), since58_irq_cnt);
		kfree(resp_buf);
		break;
	case CMD_GET_APPLICATION_INFO:
		/* live20 / identify: empty control GET only. */
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: live20 delay_ms=%u (empty GET 0x20)\n",
			saaios_exp_seq, delay_ms);
		live20 = saaios_live_0x20(tcm_hcd);
		saaios_live20 = live20;
		saaios_last_retval = live20;
		saaios_last_response = (live20 < 0) ? 0xff : 0x00;
		retval = live20;
		break;
	default:
		saaios_last_retval = -EINVAL;
		saaios_last_response = 0xff;
		retval = -EINVAL;
		break;
	}

	/* Any experiment retval<0 → dead. Do not leave state=ready (IC unknown).
	 * Do not send a follow-up live 0x20 after payload timeout (jams further).
	 */
	if (retval < 0) {
		saaios_mark_dead(retval, "experiment timeout/error");
		atomic_set(&saaios_exp_busy, 0);
		return;
	}

	saaios_state = SAAIOS_ST_READY;
	atomic_set(&saaios_exp_busy, 0);

	if (from_ladder) {
		saaios_ladder_i++;
		if (saaios_ladder_i < SAAIOS_LIVE20_LADDER_N)
			saaios_ladder_kick();
		else {
			saaios_ladder_active = 0;
			pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP: live20 ladder done all %u steps OK\n",
				SAAIOS_LIVE20_LADDER_N);
		}
	}
}

ssize_t syna_tcm_saaios_status_show(struct syna_tcm_hcd *tcm_hcd, char *buf)
{
	int attn;
	unsigned char resp;

	if (!tcm_hcd || !buf)
		return 0;
	attn = saaios_attn_gpio(tcm_hcd);
	/* Never echo stale response_code when retval<0. */
	resp = (saaios_last_retval < 0) ? 0xff : saaios_last_response;
	return scnprintf(buf, PAGE_SIZE,
		"seq=%u state=%s action=0x%02x retval=%d response=%02x\n"
		"live20=%d mode=%02x attn=%d irq=%u rx=%u report_touch=%u\n",
		saaios_exp_seq, saaios_state_str(), saaios_last_cmd,
		saaios_last_retval, resp, saaios_live20,
		tcm_hcd->id_info.mode, attn, since58_irq_cnt, since58_rx_cnt,
		saaios_report_touch_n);
}

static int saaios_queue_cmd(struct syna_tcm_hcd *tcm_hcd, unsigned int cmd,
		unsigned int delay_ms)
{
	if (!tcm_hcd || !saaios_touch_wq)
		return -ENODEV;
	if (saaios_state == SAAIOS_ST_DEAD)
		return -EBUSY;
	if (atomic_cmpxchg(&saaios_exp_busy, 0, 1) != 0)
		return -EBUSY;

	saaios_touch_tcm = tcm_hcd;
	saaios_pending_cmd = cmd;
	saaios_pending_delay_ms = delay_ms;
	saaios_exp_seq++;
	saaios_last_cmd = cmd;
	saaios_state = SAAIOS_ST_BUSY;
	pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: queued action=0x%02x delay_ms=%u (SPI on saaios_touch_wq)\n",
		saaios_exp_seq, cmd, delay_ms);
	queue_work(saaios_touch_wq, &saaios_touch_work);
	return 0;
}

static void saaios_ladder_dwork_fn(struct work_struct *work)
{
	struct syna_tcm_hcd *tcm_hcd = saaios_touch_tcm;
	unsigned int target_ms;
	s64 elapsed_ms;
	long wait_ms;

	(void)work;

	if (!saaios_ladder_active || !tcm_hcd)
		return;
	if (saaios_state == SAAIOS_ST_DEAD) {
		saaios_ladder_active = 0;
		return;
	}
	if (saaios_ladder_i >= SAAIOS_LIVE20_LADDER_N) {
		saaios_ladder_active = 0;
		return;
	}

	target_ms = saaios_live20_delays_ms[saaios_ladder_i];
	elapsed_ms = ktime_to_ms(ktime_sub(ktime_get(), saaios_reinit_ok_kt));
	if (elapsed_ms < (s64)target_ms) {
		wait_ms = (long)((s64)target_ms - elapsed_ms);
		if (wait_ms < 1)
			wait_ms = 1;
		schedule_delayed_work(&saaios_ladder_dwork,
				msecs_to_jiffies(wait_ms));
		return;
	}

	if (saaios_state == SAAIOS_ST_BUSY ||
			atomic_read(&saaios_exp_busy)) {
		schedule_delayed_work(&saaios_ladder_dwork,
				msecs_to_jiffies(5));
		return;
	}

	pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP: live20 ladder step=%u/%u target_delay_ms=%u elapsed_ms=%lld\n",
		saaios_ladder_i + 1, SAAIOS_LIVE20_LADDER_N, target_ms,
		(long long)elapsed_ms);
	if (saaios_queue_cmd(tcm_hcd, CMD_GET_APPLICATION_INFO, target_ms) < 0) {
		if (saaios_state != SAAIOS_ST_DEAD)
			schedule_delayed_work(&saaios_ladder_dwork,
					msecs_to_jiffies(5));
	}
}

static void saaios_ladder_kick(void)
{
	if (!saaios_ladder_active)
		return;
	schedule_delayed_work(&saaios_ladder_dwork, 0);
}

static void saaios_start_live20_ladder(struct syna_tcm_hcd *tcm_hcd)
{
	saaios_touch_tcm = tcm_hcd;
	saaios_reinit_ok_kt = ktime_get();
	saaios_ladder_i = 0;
	saaios_ladder_active = 1;
	pr_info("SAaiOS_TOUCH_DBG: start live20 ladder delays_ms=10,100,500,1000 after REINIT unlock (no auto 0x05/0x24/0x30; no delay=0 under reset_mutex)\n");
	schedule_delayed_work(&saaios_ladder_dwork,
			msecs_to_jiffies(saaios_live20_delays_ms[0]));
}

ssize_t syna_tcm_saaios_action_store(struct syna_tcm_hcd *tcm_hcd,
		const char *buf, size_t count)
{
	const char *p = buf;
	unsigned int cmd;
	int retval;

	if (!tcm_hcd || !buf)
		return -EINVAL;
	while (*p == ' ' || *p == '\t' || *p == '\n')
		p++;

	if (saaios_token_is(p, "live20") || saaios_token_is(p, "identify"))
		cmd = CMD_GET_APPLICATION_INFO;
	else if (saaios_token_is(p, "run_app"))
		cmd = CMD_RUN_APPLICATION_FIRMWARE;
	else if (saaios_token_is(p, "enable_report"))
		cmd = CMD_ENABLE_REPORT;
	else if (saaios_token_is(p, "no_doze"))
		cmd = CMD_SET_DYNAMIC_CONFIG;
	else if (saaios_token_is(p, "app_config"))
		/* Optional / diagnostic only — not in post-boot menu. */
		cmd = CMD_DOWNLOAD_CONFIG;
	else {
		pr_info("SAaiOS_TOUCH_DBG: saaios action unknown '%s'\n", p);
		return -EINVAL;
	}

	if (saaios_state == SAAIOS_ST_DEAD || !saaios_touch_wq)
		return -EBUSY;

	/* Manual action cancels remaining auto ladder steps. */
	saaios_ladder_active = 0;
	cancel_delayed_work_sync(&saaios_ladder_dwork);

	retval = saaios_queue_cmd(tcm_hcd, cmd, 0);
	if (retval < 0)
		return retval;
	return count;
}

static ssize_t saaios_kobj_action_store(struct kobject *kobj,
		struct kobj_attribute *attr, const char *buf, size_t count)
{
	(void)kobj;
	(void)attr;
	return syna_tcm_saaios_action_store(saaios_touch_tcm, buf, count);
}

static ssize_t saaios_kobj_status_show(struct kobject *kobj,
		struct kobj_attribute *attr, char *buf)
{
	(void)kobj;
	(void)attr;
	return syna_tcm_saaios_status_show(saaios_touch_tcm, buf);
}

static struct kobj_attribute saaios_action_kobj_attr =
	__ATTR(action, 0200, NULL, saaios_kobj_action_store);
static struct kobj_attribute saaios_status_kobj_attr =
	__ATTR(status, 0400, saaios_kobj_status_show, NULL);

int syna_tcm_saaios_exp_init(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;

	saaios_touch_tcm = tcm_hcd;
	if (saaios_touch_wq)
		return 0;
	saaios_touch_wq = alloc_ordered_workqueue("saaios_touch_wq", 0);
	if (!saaios_touch_wq)
		return -ENOMEM;
	INIT_WORK(&saaios_touch_work, saaios_touch_work_fn);
	INIT_DELAYED_WORK(&saaios_ladder_dwork, saaios_ladder_dwork_fn);

	saaios_touch_kobj = kobject_create_and_add("saaios_touch", kernel_kobj);
	if (!saaios_touch_kobj) {
		pr_err("SAaiOS_TOUCH_DBG: kobject_create_and_add saaios_touch failed\n");
		return -ENOMEM;
	}
	retval = sysfs_create_file(saaios_touch_kobj,
			&saaios_action_kobj_attr.attr);
	if (!retval)
		retval = sysfs_create_file(saaios_touch_kobj,
				&saaios_status_kobj_attr.attr);
	if (retval) {
		kobject_put(saaios_touch_kobj);
		saaios_touch_kobj = NULL;
		pr_err("SAaiOS_TOUCH_DBG: sysfs_create_file saaios_touch failed retval=%d\n",
			retval);
		return retval;
	}
	pr_info("SAaiOS_TOUCH_DBG: sysfs /sys/kernel/saaios_touch/{action,status}\n");
	return 0;
}

void syna_tcm_saaios_exp_exit(void)
{
	if (saaios_touch_kobj) {
		sysfs_remove_file(saaios_touch_kobj,
				&saaios_action_kobj_attr.attr);
		sysfs_remove_file(saaios_touch_kobj,
				&saaios_status_kobj_attr.attr);
		kobject_put(saaios_touch_kobj);
		saaios_touch_kobj = NULL;
	}
	saaios_ladder_active = 0;
	cancel_delayed_work_sync(&saaios_ladder_dwork);
	if (!saaios_touch_wq)
		return;
	cancel_work_sync(&saaios_touch_work);
	flush_workqueue(saaios_touch_wq);
	destroy_workqueue(saaios_touch_wq);
	saaios_touch_wq = NULL;
}

static void syna_tcm_since58_observe(struct syna_tcm_hcd *tcm_hcd, const char *when)
{
	int gpio = -1;

	if (!tcm_hcd)
		return;
	if (tcm_hcd->hw_if && tcm_hcd->hw_if->bdata &&
			tcm_hcd->hw_if->bdata->irq_gpio >= 0)
		gpio = gpio_get_value(tcm_hcd->hw_if->bdata->irq_gpio);

	pr_info("SAaiOS_TOUCH_DBG: since59 observe %s irq_cnt=%u rx_cnt=%u gpio=%d irq_en=%d mode=0x%02x packrat=%u part='%s' sent_0x45=%d\n",
		when ? when : "-", since58_irq_cnt, since58_rx_cnt, gpio,
		tcm_hcd->irq_enabled ? 1 : 0, tcm_hcd->id_info.mode,
		tcm_hcd->packrat_number, tcm_hcd->id_info.part_number,
		since58_sent_0x45 ? 1 : 0);
}

static int syna_tcm_sensor_detection(struct syna_tcm_hcd *tcm_hcd);
static void syna_tcm_check_hdl(struct syna_tcm_hcd *tcm_hcd,
							unsigned char id);
int syna_tcm_add_module(struct syna_tcm_module_cb *mod_cb, bool insert)
{
	struct syna_tcm_module_handler *mod_handler;

	if (!mod_pool.initialized) {
		mutex_init(&mod_pool.mutex);
		INIT_LIST_HEAD(&mod_pool.list);
		mod_pool.initialized = true;
	}

	mutex_lock(&mod_pool.mutex);

	if (insert) {
		mod_handler = kzalloc(sizeof(*mod_handler), GFP_KERNEL);
		if (!mod_handler) {
			pr_err("%s: Failed to allocate memory for mod_handler\n", __func__);
			mutex_unlock(&mod_pool.mutex);
			return -ENOMEM;
		}
		mod_handler->mod_cb = mod_cb;
		mod_handler->insert = true;
		mod_handler->detach = false;
		list_add_tail(&mod_handler->link, &mod_pool.list);
	} else if (!list_empty(&mod_pool.list)) {
		list_for_each_entry(mod_handler, &mod_pool.list, link) {
			if (mod_handler->mod_cb->type == mod_cb->type) {
				mod_handler->insert = false;
				mod_handler->detach = true;
				goto exit;
			}
		}
	}

exit:
	mutex_unlock(&mod_pool.mutex);

	if (mod_pool.queue_work)
		queue_work(mod_pool.workqueue, &mod_pool.work);

	return 0;
}
EXPORT_SYMBOL(syna_tcm_add_module);

static void syna_tcm_module_work(struct work_struct *work)
{
	struct syna_tcm_module_handler *mod_handler;
	struct syna_tcm_module_handler *tmp_handler;
	struct syna_tcm_hcd *tcm_hcd = mod_pool.tcm_hcd;

	mutex_lock(&mod_pool.mutex);

	if (!list_empty(&mod_pool.list)) {
		list_for_each_entry_safe(mod_handler, tmp_handler, &mod_pool.list, link) {
			if (mod_handler->insert) {
				if (mod_handler->mod_cb->init)
					mod_handler->mod_cb->init(tcm_hcd);
				mod_handler->insert = false;
			}
			if (mod_handler->detach) {
				if (mod_handler->mod_cb->remove)
					mod_handler->mod_cb->remove(tcm_hcd);
				list_del(&mod_handler->link);
				kfree(mod_handler);
			}
		}
	}

	mutex_unlock(&mod_pool.mutex);

	return;
}

void sec_ts_print_info(struct syna_tcm_hcd *tcm_hcd)
{
	if (!tcm_hcd)
		return;

	tcm_hcd->print_info_cnt_open++;

	if (tcm_hcd->print_info_cnt_open > 0xfff0)
		tcm_hcd->print_info_cnt_open = 0;

	if (tcm_hcd->touch_count == 0)
		tcm_hcd->print_info_cnt_release++;

	input_info(true, tcm_hcd->pdev->dev.parent,
		"tc:%d noise:%d Sensitivity:%d sip:%d game:%d // v:%02X%02X // irq:%d //#%d %d\n",
		tcm_hcd->touch_count, tcm_hcd->noise, tcm_hcd->sensitivity_mode, tcm_hcd->sip_mode,
		tcm_hcd->game_mode, tcm_hcd->app_info.customer_config_id[2], tcm_hcd->app_info.customer_config_id[3],
		gpio_get_value(tcm_hcd->hw_if->bdata->irq_gpio), tcm_hcd->print_info_cnt_open, tcm_hcd->print_info_cnt_release);
}

static void touch_print_info_work(struct work_struct *work)
{
	struct syna_tcm_hcd *tcm_hcd = container_of(work, struct syna_tcm_hcd,
			work_print_info.work);

	sec_ts_print_info(tcm_hcd);
	syna_tcm_since58_observe(tcm_hcd, "print_info");

	if (!shutdown_is_on_going_tsp)
		schedule_delayed_work(&tcm_hcd->work_print_info, msecs_to_jiffies(TOUCH_PRINT_INFO_DWORK_TIME));
}

static void sec_read_info_work(struct work_struct *work)
{
	struct syna_tcm_hcd *tcm_hcd = container_of(work, struct syna_tcm_hcd, 
			work_read_info.work);

	input_log_fix();
	sec_run_rawdata(tcm_hcd);
}

#ifdef REPORT_NOTIFIER
/**
 * syna_tcm_report_notifier() - notify occurrence of report received from device
 *
 * @data: handle of core module
 *
 * The occurrence of the report generated by the device is forwarded to the
 * asynchronous inbox of each registered application module.
 */
static int syna_tcm_report_notifier(void *data)
{
	struct sched_param param = { .sched_priority = NOTIFIER_PRIORITY };
	struct syna_tcm_module_handler *mod_handler;
	struct syna_tcm_hcd *tcm_hcd = data;

	sched_setscheduler(current, SCHED_RR, &param);

	set_current_state(TASK_INTERRUPTIBLE);

	while (!kthread_should_stop()) {
		schedule();

		if (kthread_should_stop())
			break;

		set_current_state(TASK_RUNNING);

		mutex_lock(&mod_pool.mutex);

		if (!list_empty(&mod_pool.list)) {
			list_for_each_entry(mod_handler, &mod_pool.list, link) {
				if (!mod_handler->insert && !mod_handler->detach &&
					(mod_handler->mod_cb->asyncbox))
					mod_handler->mod_cb->asyncbox(tcm_hcd);
			}
		}

		mutex_unlock(&mod_pool.mutex);

		set_current_state(TASK_INTERRUPTIBLE);
	};

	return 0;
}
#endif

/**
 * syna_tcm_dispatch_report() - dispatch report received from device
 *
 * @tcm_hcd: handle of core module
 *
 * The report generated by the device is forwarded to the synchronous inbox of
 * each registered application module for further processing. In addition, the
 * report notifier thread is woken up for asynchronous notification of the
 * report occurrence.
 */
static void syna_tcm_dispatch_report(struct syna_tcm_hcd *tcm_hcd)
{
	struct syna_tcm_module_handler *mod_handler;

	LOCK_BUFFER(tcm_hcd->in);
	LOCK_BUFFER(tcm_hcd->report.buffer);

	tcm_hcd->report.buffer.buf = &tcm_hcd->in.buf[MESSAGE_HEADER_SIZE];

	tcm_hcd->report.buffer.buf_size = tcm_hcd->in.buf_size;
	tcm_hcd->report.buffer.buf_size -= MESSAGE_HEADER_SIZE;

	tcm_hcd->report.buffer.data_length = tcm_hcd->payload_length;

	tcm_hcd->report.id = tcm_hcd->status_report_code;

	/* report directly if touch report is received */
	if (tcm_hcd->report.id == REPORT_TOUCH) {
		saaios_report_touch_n++;
		if (saaios_report_touch_n <= 4)
			pr_info("SAaiOS_TOUCH_DBG: REPORT_TOUCH 0x11 n=%u plen=%u report_touch=%d irq_cnt=%u rx_cnt=%u\n",
				saaios_report_touch_n, tcm_hcd->payload_length,
				tcm_hcd->report_touch ? 1 : 0,
				since58_irq_cnt, since58_rx_cnt);
		if (tcm_hcd->report_touch)
			tcm_hcd->report_touch();

	} else {
		/* once an identify report is received, */
		/* reinitialize touch in case any changes */
		if ((tcm_hcd->report.id == REPORT_IDENTIFY) &&
				IS_FW_MODE(tcm_hcd->id_info.mode)) {
			pr_info("SAaiOS_TOUCH_DBG: skip HELP_TOUCH_REINIT/0x25 mode=0x%02x (observe 0x45)\n",
				tcm_hcd->id_info.mode);
		}

		if (tcm_hcd->report.id == REPORT_STATUS) {
			unsigned int i, n;
			char hex[48];
			unsigned int cnt;

			n = 0;
			hex[0] = 0;
			for (i = 0; i < tcm_hcd->payload_length && i < 8; i++)
				n += scnprintf(hex + n, sizeof(hex) - n, "%s%02x",
					i ? " " : "",
					tcm_hcd->in.buf[MESSAGE_HEADER_SIZE + i]);
			cnt = ++saaios_leftover_1b_n;
			if (cnt == 1)
				pr_info("SAaiOS_TOUCH_DBG: leftover 0x1b storm snapshot sent_0x45=%d last_0x45_retval=%d wr_chunk=%u cmd=0x%02x host_downloading=%d mode=0x%02x irq_cnt=%u (0x7fffffff=0x45 never returned)\n",
					since58_sent_0x45 ? 1 : 0, saaios_last_0x45_retval,
					tcm_hcd->wr_chunk_size, tcm_hcd->command,
					atomic_read(&tcm_hcd->host_downloading),
					tcm_hcd->id_info.mode, since58_irq_cnt);
			if (cnt <= SAAIOS_LEFTOVER_LOG_N ||
					(cnt % SAAIOS_LEFTOVER_LOG_EVERY) == 0)
				pr_info("SAaiOS_TOUCH_DBG: leftover 0x1b REPORT_STATUS n=%u plen=%u payload=%s mode=0x%02x host_downloading=%d cmd=0x%02x\n",
					cnt, tcm_hcd->payload_length, hex[0] ? hex : "-",
					tcm_hcd->id_info.mode,
					atomic_read(&tcm_hcd->host_downloading),
					tcm_hcd->command);
			if (cnt == SAAIOS_IRQ_DRAIN_MAX)
				saaios_irq_emergency_hold(tcm_hcd,
					"32 leftover 0x1b");
		}

		/* dispatch received report to the other modules */
		mutex_lock(&mod_pool.mutex);

		if (!list_empty(&mod_pool.list)) {
			list_for_each_entry(mod_handler, &mod_pool.list, link) {
				if (!mod_handler->insert && !mod_handler->detach &&
						(mod_handler->mod_cb->syncbox))
					mod_handler->mod_cb->syncbox(tcm_hcd);
			}
		}

		tcm_hcd->async_report_id = tcm_hcd->status_report_code;

		mutex_unlock(&mod_pool.mutex);
	}

	UNLOCK_BUFFER(tcm_hcd->report.buffer);
	UNLOCK_BUFFER(tcm_hcd->in);

#ifdef REPORT_NOTIFIER
	wake_up_process(tcm_hcd->notifier_thread);
#endif

	return;
}

/**
 * syna_tcm_dispatch_response() - dispatch response received from device
 *
 * @tcm_hcd: handle of core module
 *
 * The response to a command is forwarded to the sender of the command.
 */
static void syna_tcm_dispatch_response(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;

	if (atomic_read(&tcm_hcd->command_status) != CMD_BUSY)
		return;

	tcm_hcd->response_code = tcm_hcd->status_report_code;

	if (tcm_hcd->payload_length == 0) {
		atomic_set(&tcm_hcd->command_status, CMD_IDLE);
		goto exit;
	}

	LOCK_BUFFER(tcm_hcd->resp);

	retval = syna_tcm_alloc_mem(tcm_hcd, &tcm_hcd->resp, tcm_hcd->payload_length);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
						"Failed to allocate memory for tcm_hcd->resp.buf\n");
		UNLOCK_BUFFER(tcm_hcd->resp);
		atomic_set(&tcm_hcd->command_status, CMD_ERROR);
		goto exit;
	}

	LOCK_BUFFER(tcm_hcd->in);

	retval = secure_memcpy(tcm_hcd->resp.buf,
				tcm_hcd->resp.buf_size, &tcm_hcd->in.buf[MESSAGE_HEADER_SIZE],
				tcm_hcd->in.buf_size - MESSAGE_HEADER_SIZE,
				tcm_hcd->payload_length);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy payload\n");
		UNLOCK_BUFFER(tcm_hcd->in);
		UNLOCK_BUFFER(tcm_hcd->resp);
		atomic_set(&tcm_hcd->command_status, CMD_ERROR);
		goto exit;
	}

	tcm_hcd->resp.data_length = tcm_hcd->payload_length;

	UNLOCK_BUFFER(tcm_hcd->in);
	UNLOCK_BUFFER(tcm_hcd->resp);

	atomic_set(&tcm_hcd->command_status, CMD_IDLE);

exit:
	complete(&response_complete);

	return;
}

/**
 * syna_tcm_dispatch_message() - dispatch message received from device
 *
 * @tcm_hcd: handle of core module
 *
 * The information received in the message read in from the device is dispatched
 * to the appropriate destination based on whether the information represents a
 * report or a response to a command.
 */
static void syna_tcm_dispatch_message(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	unsigned char *build_id;
	unsigned int payload_length;
	unsigned int max_write_size;

	since58_rx_cnt++;

	if (tcm_hcd->status_report_code == REPORT_IDENTIFY) {
		payload_length = tcm_hcd->payload_length;

		LOCK_BUFFER(tcm_hcd->in);

		syna_tcm_dump_identify(tcm_hcd, &tcm_hcd->in.buf[MESSAGE_HEADER_SIZE],
			payload_length,
			since58_sent_0x45 ? "after 0x45 (IRQ)" : "before 0x45 (IRQ leftover)");

		retval = secure_memcpy((unsigned char *)&tcm_hcd->id_info,
					sizeof(tcm_hcd->id_info), &tcm_hcd->in.buf[MESSAGE_HEADER_SIZE],
					tcm_hcd->in.buf_size - MESSAGE_HEADER_SIZE,
					MIN(sizeof(tcm_hcd->id_info), payload_length));
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to copy identification info\n");
			UNLOCK_BUFFER(tcm_hcd->in);
			return;
		}

		UNLOCK_BUFFER(tcm_hcd->in);

		build_id = tcm_hcd->id_info.build_id;
		tcm_hcd->packrat_number = le4_to_uint(build_id);

		max_write_size = le2_to_uint(tcm_hcd->id_info.max_write_size);
		tcm_hcd->wr_chunk_size = MIN(max_write_size, WR_CHUNK_SIZE);
		if (tcm_hcd->wr_chunk_size == 0)
			tcm_hcd->wr_chunk_size = max_write_size;

		input_info(true, tcm_hcd->pdev->dev.parent,
			"Received identify report (firmware mode = 0x%02x)\n", tcm_hcd->id_info.mode);

		if (atomic_read(&tcm_hcd->command_status) == CMD_BUSY) {
			if (tcm_hcd->command == CMD_ROMBOOT_DOWNLOAD &&
					tcm_hcd->id_info.mode == MODE_HOSTDOWNLOAD_FIRMWARE) {
				int attn = -1;

				if (tcm_hcd->hw_if && tcm_hcd->hw_if->bdata &&
						tcm_hcd->hw_if->bdata->irq_gpio >= 0)
					attn = gpio_get_value(tcm_hcd->hw_if->bdata->irq_gpio);
				pr_info("SAaiOS_TOUCH_DBG: IDENTIFY 0x02 completed 0x45 waiter as STATUS_OK (not leftover abort, not -EIO) part='%s' mode=0x%02x packrat=%u ATTN gpio=%d host_downloading=%d (oneshot 0x45 historically produces this IDENTIFY; not proof APP_CODE launched)\n",
					tcm_hcd->id_info.part_number,
					tcm_hcd->id_info.mode,
					tcm_hcd->packrat_number, attn,
					atomic_read(&tcm_hcd->host_downloading));
				tcm_hcd->response_code = STATUS_OK;
				atomic_set(&tcm_hcd->command_status, CMD_IDLE);
				complete(&response_complete);
			} else {
			switch (tcm_hcd->command) {
			case CMD_RESET:
			case CMD_RUN_BOOTLOADER_FIRMWARE:
			case CMD_RUN_APPLICATION_FIRMWARE:
			case CMD_ENTER_PRODUCTION_TEST_MODE:
			case CMD_ROMBOOT_RUN_BOOTLOADER_FIRMWARE:
				tcm_hcd->response_code = STATUS_OK;
				atomic_set(&tcm_hcd->command_status, CMD_IDLE);
				complete(&response_complete);
				break;
			default:
				input_info(true, tcm_hcd->pdev->dev.parent, "Device has been reset\n");
				pr_info("SAaiOS_TOUCH_DBG: IDENTIFY abort waiter command=0x%02x (not STATUS_OK) mode=0x%02x\n",
					tcm_hcd->command, tcm_hcd->id_info.mode);
				atomic_set(&tcm_hcd->command_status, CMD_ERROR);
				complete(&response_complete);
				break;
			}
			}
		} else {

			if ((tcm_hcd->id_info.mode == MODE_ROMBOOTLOADER) && tcm_hcd->in_hdl_mode) {
				if (tcm_hcd->romboot_download_deferred) {
					pr_info("SAaiOS_TOUCH_DBG: skip HELP_SEND_ROMBOOT_HDL (0x45 deferred until panel)\n");
				} else if (atomic_read(&tcm_hcd->helper.task) == HELP_NONE) {
					atomic_set(&tcm_hcd->helper.task, HELP_SEND_ROMBOOT_HDL);
					queue_work(tcm_hcd->helper.workqueue, &tcm_hcd->helper.work);
				} else {
					input_info(true, tcm_hcd->pdev->dev.parent, "Helper thread is busy\n");
				}
				return;
			}
		}

#ifdef FORCE_RUN_APPLICATION_FIRMWARE
		if (IS_NOT_FW_MODE(tcm_hcd->id_info.mode) && !mutex_is_locked(&tcm_hcd->reset_mutex)) {
			if (atomic_read(&tcm_hcd->helper.task) == HELP_NONE) {
				atomic_set(&tcm_hcd->helper.task, HELP_RUN_APPLICATION_FIRMWARE);
				queue_work(tcm_hcd->helper.workqueue, &tcm_hcd->helper.work);
				return;
			}
		}
#endif

		/* To avoid the identify report dispatching during the HDL. */
		if (atomic_read(&tcm_hcd->host_downloading)) {
			input_info(true, tcm_hcd->pdev->dev.parent,
				"Switched to TCM mode and going to download the configs\n");
			return;
		}
	}

	if (tcm_hcd->status_report_code >= REPORT_IDENTIFY)
		syna_tcm_dispatch_report(tcm_hcd);
	else
		syna_tcm_dispatch_response(tcm_hcd);

	return;
}

/**
 * syna_tcm_continued_read() - retrieve entire payload from device
 *
 * @tcm_hcd: handle of core module
 *
 * Read transactions are carried out until the entire payload is retrieved from
 * the device and stored in the handle of the core module.
 */
static int syna_tcm_continued_read(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	unsigned char marker;
	unsigned char code;
	unsigned int idx;
	unsigned int offset;
	unsigned int chunks;
	unsigned int chunk_space;
	unsigned int xfer_length;
	unsigned int total_length;
	unsigned int remaining_length;

	total_length = MESSAGE_HEADER_SIZE + tcm_hcd->payload_length + 1;

	remaining_length = total_length - tcm_hcd->read_length;

	LOCK_BUFFER(tcm_hcd->in);

	retval = syna_tcm_realloc_mem(tcm_hcd, &tcm_hcd->in, total_length + 1);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to reallocate memory for tcm_hcd->in.buf\n");
		UNLOCK_BUFFER(tcm_hcd->in);
		return retval;
	}

	/* available chunk space for payload = total chunk size minus header
	 * marker byte and header code byte */
	if (tcm_hcd->rd_chunk_size == 0)
		chunk_space = remaining_length;
	else
		chunk_space = tcm_hcd->rd_chunk_size - 2;

	chunks = ceil_div(remaining_length, chunk_space);

	chunks = chunks == 0 ? 1 : chunks;

	offset = tcm_hcd->read_length;

	LOCK_BUFFER(tcm_hcd->temp);

	for (idx = 0; idx < chunks; idx++) {
		if (remaining_length > chunk_space)
			xfer_length = chunk_space;
		else
			xfer_length = remaining_length;

		if (xfer_length == 1) {
			tcm_hcd->in.buf[offset] = MESSAGE_PADDING;
			offset += xfer_length;
			remaining_length -= xfer_length;
			continue;
		}

		retval = syna_tcm_alloc_mem(tcm_hcd, &tcm_hcd->temp, xfer_length + 2);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to allocate memory for tcm_hcd->temp.buf\n");
			UNLOCK_BUFFER(tcm_hcd->temp);
			UNLOCK_BUFFER(tcm_hcd->in);
			return retval;
		}

		retval = syna_tcm_read(tcm_hcd,	tcm_hcd->temp.buf, xfer_length + 2);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to read from device\n");
			UNLOCK_BUFFER(tcm_hcd->temp);
			UNLOCK_BUFFER(tcm_hcd->in);
			return retval;
		}

		marker = tcm_hcd->temp.buf[0];
		code = tcm_hcd->temp.buf[1];

		if (marker != MESSAGE_MARKER) {
			pr_info("SAaiOS_TOUCH_DBG: 0x%02x continued-read fail marker=0x%02x plen=%u first_read=%u\n",
				tcm_hcd->command, marker,
				tcm_hcd->payload_length, tcm_hcd->read_length);
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Incorrect header marker (0x%02x)\n", marker);
			UNLOCK_BUFFER(tcm_hcd->temp);
			UNLOCK_BUFFER(tcm_hcd->in);
			return -EIO;
		}

		if (code != STATUS_CONTINUED_READ) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Incorrect header code (0x%02x)\n", code);
			UNLOCK_BUFFER(tcm_hcd->temp);
			UNLOCK_BUFFER(tcm_hcd->in);
			return -EIO;
		}

		retval = secure_memcpy(&tcm_hcd->in.buf[offset],
					tcm_hcd->in.buf_size - offset, &tcm_hcd->temp.buf[2],
					tcm_hcd->temp.buf_size - 2,	xfer_length);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy payload\n");
			UNLOCK_BUFFER(tcm_hcd->temp);
			UNLOCK_BUFFER(tcm_hcd->in);
			return retval;
		}

		offset += xfer_length;

		remaining_length -= xfer_length;
	}

	UNLOCK_BUFFER(tcm_hcd->temp);
	UNLOCK_BUFFER(tcm_hcd->in);

	return 0;
}

/**
 * syna_tcm_raw_read() - retrieve specific number of data bytes from device
 *
 * @tcm_hcd: handle of core module
 * @in_buf: buffer for storing data retrieved from device
 * @length: number of bytes to retrieve from device
 *
 * Read transactions are carried out until the specific number of data bytes are
 * retrieved from the device and stored in in_buf.
 */
static int syna_tcm_raw_read(struct syna_tcm_hcd *tcm_hcd,
		unsigned char *in_buf, unsigned int length)
{
	int retval;
	unsigned char code;
	unsigned int idx;
	unsigned int offset;
	unsigned int chunks;
	unsigned int chunk_space;
	unsigned int xfer_length;
	unsigned int remaining_length;

	if (length < 2) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Invalid length information\n");
		return -EINVAL;
	}

	/* minus header marker byte and header code byte */
	remaining_length = length - 2;

	/* available chunk space for data = total chunk size minus header marker
	 * byte and header code byte */
	if (tcm_hcd->rd_chunk_size == 0)
		chunk_space = remaining_length;
	else
		chunk_space = tcm_hcd->rd_chunk_size - 2;

	chunks = ceil_div(remaining_length, chunk_space);

	chunks = chunks == 0 ? 1 : chunks;

	offset = 0;

	LOCK_BUFFER(tcm_hcd->temp);

	for (idx = 0; idx < chunks; idx++) {
		if (remaining_length > chunk_space)
			xfer_length = chunk_space;
		else
			xfer_length = remaining_length;

		if (xfer_length == 1) {
			in_buf[offset] = MESSAGE_PADDING;
			offset += xfer_length;
			remaining_length -= xfer_length;
			continue;
		}

		retval = syna_tcm_alloc_mem(tcm_hcd, &tcm_hcd->temp, xfer_length + 2);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to allocate memory for tcm_hcd->temp.buf\n");
			UNLOCK_BUFFER(tcm_hcd->temp);
			return retval;
		}

		retval = syna_tcm_read(tcm_hcd,	tcm_hcd->temp.buf, xfer_length + 2);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to read from device\n");
			UNLOCK_BUFFER(tcm_hcd->temp);
			return retval;
		}

		code = tcm_hcd->temp.buf[1];

		if (idx == 0) {
			retval = secure_memcpy(&in_buf[0], length,
						&tcm_hcd->temp.buf[0], tcm_hcd->temp.buf_size,
						xfer_length + 2);
		} else {
			if (code != STATUS_CONTINUED_READ) {
				input_err(true, tcm_hcd->pdev->dev.parent,
					"Incorrect header code (0x%02x)\n",	code);
				UNLOCK_BUFFER(tcm_hcd->temp);
				return -EIO;
			}

			retval = secure_memcpy(&in_buf[offset],
						length - offset, &tcm_hcd->temp.buf[2],
						tcm_hcd->temp.buf_size - 2, xfer_length);
		}
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy data\n");
			UNLOCK_BUFFER(tcm_hcd->temp);
			return retval;
		}

		if (idx == 0)
			offset += (xfer_length + 2);
		else
			offset += xfer_length;

		remaining_length -= xfer_length;
	}

	UNLOCK_BUFFER(tcm_hcd->temp);

	return 0;
}

/**
 * syna_tcm_raw_write() - write command/data to device without receiving
 * response
 *
 * @tcm_hcd: handle of core module
 * @command: command to send to device
 * @data: data to send to device
 * @length: length of data in bytes
 *
 * A command and its data, if any, are sent to the device.
 */
static int syna_tcm_raw_write(struct syna_tcm_hcd *tcm_hcd,
		unsigned char command, unsigned char *data, unsigned int length)
{
	int retval;
	unsigned int idx;
	unsigned int chunks;
	unsigned int chunk_space;
	unsigned int xfer_length;
	unsigned int remaining_length;

	remaining_length = length;

	/* available chunk space for data = total chunk size minus command
	 * byte */
	if (tcm_hcd->wr_chunk_size == 0)
		chunk_space = remaining_length;
	else
		chunk_space = tcm_hcd->wr_chunk_size - 1;

	chunks = ceil_div(remaining_length, chunk_space);

	chunks = chunks == 0 ? 1 : chunks;

	LOCK_BUFFER(tcm_hcd->out);

	for (idx = 0; idx < chunks; idx++) {
		if (remaining_length > chunk_space)
			xfer_length = chunk_space;
		else
			xfer_length = remaining_length;

		retval = syna_tcm_alloc_mem(tcm_hcd, &tcm_hcd->out, xfer_length + 1);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to allocate memory for tcm_hcd->out.buf\n");
			UNLOCK_BUFFER(tcm_hcd->out);
			return retval;
		}

		if (idx == 0)
			tcm_hcd->out.buf[0] = command;
		else
			tcm_hcd->out.buf[0] = CMD_CONTINUE_WRITE;

		if (xfer_length) {
			retval = secure_memcpy(&tcm_hcd->out.buf[1],
						tcm_hcd->out.buf_size - 1, &data[idx * chunk_space],
						remaining_length, xfer_length);
			if (retval < 0) {
				input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy data\n");
				UNLOCK_BUFFER(tcm_hcd->out);
				return retval;
			}
		}

		retval = syna_tcm_write(tcm_hcd, tcm_hcd->out.buf, xfer_length + 1);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to write to device\n");
			UNLOCK_BUFFER(tcm_hcd->out);
			return retval;
		}

		remaining_length -= xfer_length;
	}

	UNLOCK_BUFFER(tcm_hcd->out);

	return 0;
}

/**
 * syna_tcm_read_message() - read message from device
 *
 * @tcm_hcd: handle of core module
 * @in_buf: buffer for storing data in raw read mode
 * @length: length of data in bytes in raw read mode
 *
 * If in_buf is not NULL, raw read mode is used and syna_tcm_raw_read() is
 * called. Otherwise, a message including its entire payload is retrieved from
 * the device and dispatched to the appropriate destination.
 */
static int syna_tcm_read_message(struct syna_tcm_hcd *tcm_hcd,
		unsigned char *in_buf, unsigned int length)
{
	int retval;
	bool retry;
	unsigned int total_length;
	struct syna_tcm_message_header *header;

	if (tcm_hcd->lp_state == PWR_OFF) {
		input_err(true, tcm_hcd->pdev->dev.parent, "power off in suspend\n");
		return -EIO;
	}

	mutex_lock(&tcm_hcd->rw_ctrl_mutex);

	if (in_buf != NULL) {
		retval = syna_tcm_raw_read(tcm_hcd, in_buf, length);
		goto exit;
	}

	retry = true;

retry:
	LOCK_BUFFER(tcm_hcd->in);

	/* 0x25-only floor: force the first read of a GET_TOUCH_REPORT_CONFIG
	 * response to at least SAAIOS_TRC_READ_LEN (4+128+1=133) so it is not
	 * left at a short read_length carried over from the prior message
	 * (e.g. 51 after a 0x20). A short first read here desyncs SPI and
	 * syna_tcm_continued_read() sees marker 0x25 instead of 0xA5. Do not
	 * reintroduce the old global per-message clamp (broke plain 0x20).
	 */
	if (tcm_hcd->command == CMD_GET_TOUCH_REPORT_CONFIG &&
			tcm_hcd->read_length < SAAIOS_TRC_READ_LEN)
		tcm_hcd->read_length = SAAIOS_TRC_READ_LEN;

	/* Grow in.buf before a floor-sized first-read. Probe used to alloc
	 * MIN_READ_LENGTH+1=10; a 256-byte syna_tcm_read would overflow.
	 */
	retval = syna_tcm_realloc_mem(tcm_hcd, &tcm_hcd->in,
			tcm_hcd->read_length + 1);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to reallocate memory for tcm_hcd->in.buf\n");
		UNLOCK_BUFFER(tcm_hcd->in);
		goto exit;
	}

	retval = syna_tcm_read(tcm_hcd, tcm_hcd->in.buf, tcm_hcd->read_length);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to read from device\n");
		UNLOCK_BUFFER(tcm_hcd->in);
		if (retry) {
			usleep_range(READ_RETRY_US_MIN, READ_RETRY_US_MAX);
			retry = false;
			goto retry;
		}
		goto exit;
	}

	header = (struct syna_tcm_message_header *)tcm_hcd->in.buf;

	if (header->marker != MESSAGE_MARKER) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Incorrect header marker (0x%02x)\n", header->marker);
		UNLOCK_BUFFER(tcm_hcd->in);
		retval = -ENXIO;
		if (retry) {
			usleep_range(READ_RETRY_US_MIN, READ_RETRY_US_MAX);
			retry = false;
			goto retry;
		}
		goto exit;
	}

	tcm_hcd->status_report_code = header->code;

	tcm_hcd->payload_length = le2_to_uint(header->length);

	if (atomic_read(&tcm_hcd->host_downloading)) 
		input_info(true, tcm_hcd->pdev->dev.parent,
				"Status report code = 0x%02x\n", tcm_hcd->status_report_code);
	else
		input_dbg(false, tcm_hcd->pdev->dev.parent,
				"Status report code = 0x%02x\n", tcm_hcd->status_report_code);

	input_dbg(false, tcm_hcd->pdev->dev.parent,
				"Payload length = %d\n", tcm_hcd->payload_length);

	if (tcm_hcd->status_report_code <= STATUS_ERROR ||
		tcm_hcd->status_report_code == STATUS_INVALID) {
		switch (tcm_hcd->status_report_code) {
		case STATUS_OK:
			break;
		case STATUS_CONTINUED_READ:
			input_dbg(true, tcm_hcd->pdev->dev.parent, "Out-of-sync continued read\n");
		case STATUS_IDLE:
		case STATUS_BUSY:
			tcm_hcd->payload_length = 0;
			UNLOCK_BUFFER(tcm_hcd->in);
			retval = 0;
			goto exit;
		default:
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Incorrect Status code (0x%02x)\n", tcm_hcd->status_report_code);
			if (tcm_hcd->status_report_code == STATUS_INVALID) {
				if (retry) {
					usleep_range(READ_RETRY_US_MIN, READ_RETRY_US_MAX);
					retry = false;
					goto retry;
				} else {
					tcm_hcd->payload_length = 0;
				}
			}
		}
	}

	total_length = MESSAGE_HEADER_SIZE + tcm_hcd->payload_length + 1;

#ifdef PREDICTIVE_READING
	if (total_length <= tcm_hcd->read_length) {
		goto check_padding;
	} else if (total_length - 1 == tcm_hcd->read_length) {
		tcm_hcd->in.buf[total_length - 1] = MESSAGE_PADDING;
		goto check_padding;
	}
#else
	if (tcm_hcd->payload_length == 0) {
		tcm_hcd->in.buf[total_length - 1] = MESSAGE_PADDING;
		goto check_padding;
	}
#endif
	if (tcm_hcd->command == CMD_GET_TOUCH_REPORT_CONFIG)
		pr_info("SAaiOS_TOUCH_DBG: 0x25 need continued-read first_read=%u plen=%u total=%u\n",
			tcm_hcd->read_length, tcm_hcd->payload_length,
			total_length);

	UNLOCK_BUFFER(tcm_hcd->in);

	retval = syna_tcm_continued_read(tcm_hcd);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to do continued read\n");
		goto exit;
	};

	LOCK_BUFFER(tcm_hcd->in);

	tcm_hcd->in.buf[0] = MESSAGE_MARKER;
	tcm_hcd->in.buf[1] = tcm_hcd->status_report_code;
	tcm_hcd->in.buf[2] = (unsigned char)tcm_hcd->payload_length;
	tcm_hcd->in.buf[3] = (unsigned char)(tcm_hcd->payload_length >> 8);

check_padding:
	if (tcm_hcd->in.buf[total_length - 1] != MESSAGE_PADDING) {
		if (tcm_hcd->command == CMD_GET_TOUCH_REPORT_CONFIG)
			pr_info("SAaiOS_TOUCH_DBG: padding=0x%02x at total-1 (total=%u first_read=%u cmd=0x%02x)\n",
				tcm_hcd->in.buf[total_length - 1], total_length,
				tcm_hcd->read_length, tcm_hcd->command);
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Incorrect message padding byte (0x%02x)\n", tcm_hcd->in.buf[total_length - 1]);
		UNLOCK_BUFFER(tcm_hcd->in);
		retval = -EIO;
		goto exit;
	}

	UNLOCK_BUFFER(tcm_hcd->in);

#ifdef PREDICTIVE_READING
	total_length = MAX(total_length, MIN_READ_LENGTH);
	tcm_hcd->read_length = MIN(total_length, tcm_hcd->rd_chunk_size);
	if (tcm_hcd->rd_chunk_size == 0)
		tcm_hcd->read_length = total_length;
#endif
	/* No global SAAIOS_RD_FLOOR — over-read after 0x20 jammed later cmds. */
	if (tcm_hcd->is_detected)
		syna_tcm_dispatch_message(tcm_hcd);

	retval = 0;

exit:
	if (retval < 0) {
		if (atomic_read(&tcm_hcd->command_status) == CMD_BUSY) {
			atomic_set(&tcm_hcd->command_status, CMD_ERROR);
			complete(&response_complete);
		}
	}

	mutex_unlock(&tcm_hcd->rw_ctrl_mutex);

	return retval;
}

/**
 * syna_tcm_write_message() - write message to device and receive response
 *
 * @tcm_hcd: handle of core module
 * @command: command to send to device
 * @payload: payload of command
 * @length: length of payload in bytes
 * @resp_buf: buffer for storing command response
 * @resp_buf_size: size of response buffer in bytes
 * @resp_length: length of command response in bytes
 * @response_code: status code returned in command response
 * @polling_delay_ms: delay time after sending command before resuming polling
 *
 * If resp_buf is NULL, raw write mode is used and syna_tcm_raw_write() is
 * called. Otherwise, a command and its payload, if any, are sent to the device
 * and the response to the command generated by the device is read in.
 */
static int syna_tcm_write_message(struct syna_tcm_hcd *tcm_hcd,
		unsigned char command, unsigned char *payload,
		unsigned int length, unsigned char **resp_buf,
		unsigned int *resp_buf_size, unsigned int *resp_length,
		unsigned char *response_code, unsigned int polling_delay_ms)
{
	int retval;
	unsigned int idx;
	unsigned int chunks;
	unsigned int chunk_space;
	unsigned int xfer_length;
	unsigned int remaining_length;
	unsigned int command_status;
	unsigned int spi_actual = 0;
	unsigned int remaining_at_start = 0;
	bool is_romboot_hdl = (command == CMD_ROMBOOT_DOWNLOAD) ? true : false;
	bool is_hdl_reset = (command == CMD_RESET) && (tcm_hcd->in_hdl_mode);

	if (tcm_hcd->lp_state == PWR_OFF) {
		input_err(true, tcm_hcd->pdev->dev.parent, "power off in suspend\n");
		return -EIO;
	}

	if (response_code != NULL)
		*response_code = STATUS_INVALID;
	if (resp_length != NULL)
		*resp_length = 0; /* IDENTIFY-abort used to leave this uninitialized */

	if (!tcm_hcd->do_polling && current->pid == tcm_hcd->isr_pid) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Invalid execution context\n");
		return -EINVAL;
	}

	mutex_lock(&tcm_hcd->command_mutex);

	mutex_lock(&tcm_hcd->rw_ctrl_mutex);

	if (resp_buf == NULL) {
		retval = syna_tcm_raw_write(tcm_hcd, command, payload, length);
		mutex_unlock(&tcm_hcd->rw_ctrl_mutex);
		goto exit;
	}

	if (tcm_hcd->do_polling && polling_delay_ms) {
		cancel_delayed_work_sync(&tcm_hcd->polling_work);
		flush_workqueue(tcm_hcd->polling_workqueue);
	}

	atomic_set(&tcm_hcd->command_status, CMD_BUSY);

#if (LINUX_VERSION_CODE >= KERNEL_VERSION(3, 13, 0))
	reinit_completion(&response_complete);
#else
	INIT_COMPLETION(response_complete);
#endif

	tcm_hcd->command = command;

	LOCK_BUFFER(tcm_hcd->resp);

	tcm_hcd->resp.buf = *resp_buf;
	tcm_hcd->resp.buf_size = *resp_buf_size;
	tcm_hcd->resp.data_length = 0;

	UNLOCK_BUFFER(tcm_hcd->resp);

	/* adding two length bytes as part of payload */
	remaining_length = length + 2;
	remaining_at_start = remaining_length;

	/* available chunk space for payload = total chunk size minus command
	 * byte */
	if (tcm_hcd->wr_chunk_size == 0)
		chunk_space = remaining_length;
	else
		chunk_space = tcm_hcd->wr_chunk_size - 1;

	if (is_romboot_hdl) {
		since58_sent_0x45 = true;
		/* wr_chunk_size is forced to HDL_WR_CHUNK_SIZE (0) only around
		 * this write_message call. 0 → chunk_space = remaining_length
		 * (one-shot). CONTINUE_WRITE honors the 16-bit TCM length
		 * (0x7c0e) and breaks the extended 0x45 (payload=0x17c0e).
		 */
		if (HDL_WR_CHUNK_SIZE) {
			chunk_space = HDL_WR_CHUNK_SIZE - 1;
			chunk_space = chunk_space - (chunk_space % ROMBOOT_DOWNLOAD_UNIT);
		}
	}

	chunks = ceil_div(remaining_length, chunk_space);

	chunks = chunks == 0 ? 1 : chunks;
	saaios_0x45_chunks = is_romboot_hdl ? chunks : 0;
	saaios_0x45_chunk_idx = 0;

	if (is_romboot_hdl && payload && length > 14 + 15)
		pr_info("SAaiOS_TOUCH_DBG: 0x45 TX APP_CODE[0]=0x%02x first16=%02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x (write_message payload ptr, reserved=14, APP_CODE at payload[14], expect 55 aa 01 00)\n",
			payload[14 + 0],
			payload[14], payload[15], payload[16], payload[17],
			payload[18], payload[19], payload[20], payload[21],
			payload[22], payload[23], payload[24], payload[25],
			payload[26], payload[27], payload[28], payload[29]);

	input_info(true, tcm_hcd->pdev->dev.parent, "Command = 0x%02x\n", command);
	if (command == CMD_ROMBOOT_RUN_BOOTLOADER_FIRMWARE)
		pr_info("SAaiOS_TOUCH_DBG: Command 0x42 from %s\n",
			"syna_tcm_run_bootloader_firmware");
	if (is_romboot_hdl) {
		unsigned int spi_first;
		unsigned int last_rem;
		unsigned int spi_last;
		unsigned int max_write;

		spi_first = (remaining_length > chunk_space ? chunk_space :
				remaining_length) + 1;
		if (chunks <= 1)
			last_rem = remaining_length;
		else
			last_rem = remaining_length - (chunks - 1) * chunk_space;
		spi_last = last_rem + 1;
		max_write = le2_to_uint(tcm_hcd->id_info.max_write_size);
		pr_info("SAaiOS_TOUCH_DBG: 0x45 write_message payload=%u remaining_length=%u chunk_space=%u chunks=%u HDL_WR_CHUNK_SIZE=%u wr_chunk=%u spi_first=%u spi_last=%u mode=0x%02x\n",
			length, remaining_length, chunk_space, chunks,
			HDL_WR_CHUNK_SIZE, tcm_hcd->wr_chunk_size,
			spi_first, spi_last, tcm_hcd->id_info.mode);
		pr_info("SAaiOS_TOUCH_DBG: 0x45 chunk math wr_chunk=%u chunk_space=%u chunks=%u spi_first=%u spi_last=%u remaining_start=%u max_write=%u WR_CHUNK_SIZE=%u (oneshot expect wr_chunk=0 chunks=1 spi_len=97297)\n",
			tcm_hcd->wr_chunk_size, chunk_space, chunks, spi_first,
			spi_last, remaining_length, max_write, WR_CHUNK_SIZE);
		if (tcm_hcd->wr_chunk_size != 0 || chunks != 1 ||
				spi_first != remaining_length + 1)
			pr_info("SAaiOS_TOUCH_DBG: 0x45 chunk math differs from oneshot expectation (not faking) wr_chunk=%u HDL_WR_CHUNK_SIZE=%u chunks=%u spi_first=%u max_write=%u\n",
				tcm_hcd->wr_chunk_size, HDL_WR_CHUNK_SIZE, chunks,
				spi_first, max_write);
	}
	if (command == CMD_DOWNLOAD_CONFIG) {
		unsigned int spi_first;

		spi_first = (remaining_length > chunk_space ? chunk_space :
				remaining_length) + 1;
		pr_info("SAaiOS_TOUCH_DBG TOUCH_EXP[%u]: 0x30 write_message payload=%u remaining_length=%u chunk_space=%u chunks=%u wr_chunk=%u spi_first=%u (oneshot expect wr_chunk=0 chunks=1 spi_len≈4100+)\n",
			syna_tcm_saaios_exp_seq(), length, remaining_length,
			chunk_space, chunks, tcm_hcd->wr_chunk_size, spi_first);
	}

	LOCK_BUFFER(tcm_hcd->out);

	for (idx = 0; idx < chunks; idx++) {
		if (remaining_length > chunk_space)
			xfer_length = chunk_space;
		else
			xfer_length = remaining_length;

		retval = syna_tcm_alloc_mem(tcm_hcd, &tcm_hcd->out, xfer_length + 1);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to allocate memory for tcm_hcd->out.buf\n");
			UNLOCK_BUFFER(tcm_hcd->out);
			mutex_unlock(&tcm_hcd->rw_ctrl_mutex);
			goto exit;
		}

		if (idx == 0) {
			tcm_hcd->out.buf[0] = command;
			tcm_hcd->out.buf[1] = (unsigned char)length;
			tcm_hcd->out.buf[2] = (unsigned char)(length >> 8);

			if (xfer_length > 2) {
				retval = secure_memcpy(&tcm_hcd->out.buf[3],
							tcm_hcd->out.buf_size - 3, payload,
							remaining_length - 2, xfer_length - 2);
				if (retval < 0) {
					input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy payload\n");
					UNLOCK_BUFFER(tcm_hcd->out);
					mutex_unlock(&tcm_hcd->rw_ctrl_mutex);
					goto exit;
				}
			}
		} else {
			tcm_hcd->out.buf[0] = CMD_CONTINUE_WRITE;

			retval = secure_memcpy(&tcm_hcd->out.buf[1],
						tcm_hcd->out.buf_size - 1, &payload[idx * chunk_space - 2],
						remaining_length, xfer_length);
			if (retval < 0) {
				input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy payload\n");
				UNLOCK_BUFFER(tcm_hcd->out);
				mutex_unlock(&tcm_hcd->rw_ctrl_mutex);
				goto exit;
			}
		}

		if (is_romboot_hdl) {
			saaios_0x45_chunk_idx = idx;
			if (idx == 0 || idx == 1 || idx + 1 == chunks) {
				const char *which;

				which = idx == 0 ? "first" : (idx == 1 ? "second" : "last");
				pr_info("SAaiOS_TOUCH_DBG: 0x45 SPI %s idx=%u/%u cmd=0x%02x spi_len=%u remaining_length=%u first16=%02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x (0x45 vs CONTINUE_WRITE=0x01; IDENTIFY here is not firmware-started)\n",
					which, idx, chunks, tcm_hcd->out.buf[0],
					xfer_length + 1, remaining_length,
					tcm_hcd->out.buf[0], tcm_hcd->out.buf[1],
					tcm_hcd->out.buf[2], tcm_hcd->out.buf[3],
					tcm_hcd->out.buf[4], tcm_hcd->out.buf[5],
					tcm_hcd->out.buf[6], tcm_hcd->out.buf[7],
					tcm_hcd->out.buf[8], tcm_hcd->out.buf[9],
					tcm_hcd->out.buf[10], tcm_hcd->out.buf[11],
					tcm_hcd->out.buf[12], tcm_hcd->out.buf[13],
					tcm_hcd->out.buf[14], tcm_hcd->out.buf[15]);
			}
		}

		retval = syna_tcm_write(tcm_hcd, tcm_hcd->out.buf, xfer_length + 1);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to write to device\n");
			UNLOCK_BUFFER(tcm_hcd->out);
			mutex_unlock(&tcm_hcd->rw_ctrl_mutex);
			goto exit;
		}

		spi_actual += xfer_length + 1;
		remaining_length -= xfer_length;

		if (chunks > 1)
			usleep_range(WRITE_DELAY_US_MIN, WRITE_DELAY_US_MAX);
	}

	UNLOCK_BUFFER(tcm_hcd->out);

	mutex_unlock(&tcm_hcd->rw_ctrl_mutex);

	if (is_romboot_hdl)
		pr_info("SAaiOS_TOUCH_DBG: 0x45 SPI done actual=%u remaining_length=%u remaining_start=%u chunks=%u\n",
			spi_actual, remaining_length, remaining_at_start, chunks);

	if (is_hdl_reset)
		goto exit;

	if (tcm_hcd->do_polling && polling_delay_ms) {
		queue_delayed_work(tcm_hcd->polling_workqueue, &tcm_hcd->polling_work,
			msecs_to_jiffies(polling_delay_ms));
	}

	retval = wait_for_completion_timeout(&response_complete,
					msecs_to_jiffies(RESPONSE_TIMEOUT_MS));
	if (is_romboot_hdl) {
		int attn = -1;

		if (tcm_hcd->hw_if && tcm_hcd->hw_if->bdata &&
				tcm_hcd->hw_if->bdata->irq_gpio >= 0)
			attn = gpio_get_value(tcm_hcd->hw_if->bdata->irq_gpio);
		pr_info("SAaiOS_TOUCH_DBG: 0x45 wait done timeout=%d wait_retval=%d cmd_status=%d response_code=0x%02x leftover_report=0x%02x mode=0x%02x ATTN gpio=%d\n",
			retval == 0 ? 1 : 0, retval,
			atomic_read(&tcm_hcd->command_status),
			tcm_hcd->response_code, tcm_hcd->status_report_code,
			tcm_hcd->id_info.mode, attn);
	}
	if (retval == 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Timed out waiting for response (command 0x%02x)\n", tcm_hcd->command);
		if (is_romboot_hdl)
			pr_info("SAaiOS_TOUCH_DBG: 0x45 return reason=timeout -ETIME\n");
		else if (tcm_hcd->command == CMD_GET_APPLICATION_INFO ||
				tcm_hcd->command == CMD_GET_TOUCH_REPORT_CONFIG ||
				tcm_hcd->command == CMD_ENABLE_REPORT)
			pr_info("SAaiOS_TOUCH_DBG: Command 0x%02x timeout -ETIME RESPONSE_TIMEOUT_MS=%d mode=0x%02x\n",
				tcm_hcd->command, RESPONSE_TIMEOUT_MS, tcm_hcd->id_info.mode);
		retval = -ETIME;
		goto exit;
	}

	command_status = atomic_read(&tcm_hcd->command_status);
	if (command_status != CMD_IDLE) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to get valid response (command 0x%02x)\n", tcm_hcd->command);
		if (is_romboot_hdl)
			pr_info("SAaiOS_TOUCH_DBG: 0x45 return reason=%s cmd_status=%d leftover_report=0x%02x mode=0x%02x (IDENTIFY abort vs CMD_ERROR)\n",
				command_status == CMD_ERROR ? "IDENTIFY_abort_CMD_ERROR" : "not_IDLE",
				command_status, tcm_hcd->status_report_code,
				tcm_hcd->id_info.mode);
		retval = -EIO;
		goto exit;
	}

	LOCK_BUFFER(tcm_hcd->resp);

	if (tcm_hcd->response_code != STATUS_OK) {
		if (tcm_hcd->resp.data_length) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Error code = 0x%02x (command 0x%02x)\n",
				tcm_hcd->resp.buf[0],tcm_hcd->command);
		}
		if (is_romboot_hdl)
			pr_info("SAaiOS_TOUCH_DBG: 0x45 return reason=not_STATUS_OK response_code=0x%02x\n",
				tcm_hcd->response_code);
		retval = -EIO;
	} else {
		if (is_romboot_hdl)
			pr_info("SAaiOS_TOUCH_DBG: 0x45 return reason=STATUS_OK\n");
		retval = 0;
	}

	*resp_buf = tcm_hcd->resp.buf;
	*resp_buf_size = tcm_hcd->resp.buf_size;
	*resp_length = tcm_hcd->resp.data_length;

	if (response_code != NULL)
		*response_code = tcm_hcd->response_code;

	UNLOCK_BUFFER(tcm_hcd->resp);

exit:
	tcm_hcd->command = CMD_NONE;

	atomic_set(&tcm_hcd->command_status, CMD_IDLE);

	mutex_unlock(&tcm_hcd->command_mutex);

	if (is_romboot_hdl)
		saaios_last_0x45_retval = retval;

	return retval;
}

static int syna_tcm_wait_hdl(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;

	msleep(HOST_DOWNLOAD_WAIT_MS);

	if (!atomic_read(&tcm_hcd->host_downloading))
		return 0;

	retval = wait_event_interruptible_timeout(tcm_hcd->hdl_wq,
		!atomic_read(&tcm_hcd->host_downloading), msecs_to_jiffies(HOST_DOWNLOAD_TIMEOUT_MS));
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

static void syna_tcm_check_hdl(struct syna_tcm_hcd *tcm_hcd, unsigned char id)
{
	struct syna_tcm_module_handler *mod_handler;

	LOCK_BUFFER(tcm_hcd->report.buffer);

	tcm_hcd->report.buffer.buf = NULL;
	tcm_hcd->report.buffer.buf_size = 0;
	tcm_hcd->report.buffer.data_length = 0;
	tcm_hcd->report.id = id;

	UNLOCK_BUFFER(tcm_hcd->report.buffer);

	mutex_lock(&mod_pool.mutex);

	if (!list_empty(&mod_pool.list)) {
		list_for_each_entry(mod_handler, &mod_pool.list, link) {
			if (!mod_handler->insert && !mod_handler->detach && mod_handler->mod_cb->syncbox)
				mod_handler->mod_cb->syncbox(tcm_hcd);
		}
	}

	mutex_unlock(&mod_pool.mutex);

	return;
}

#ifdef WATCHDOG_SW
static void syna_tcm_update_watchdog(struct syna_tcm_hcd *tcm_hcd, bool en)
{
	cancel_delayed_work_sync(&tcm_hcd->watchdog.work);
	flush_workqueue(tcm_hcd->watchdog.workqueue);

	if (!tcm_hcd->watchdog.run) {
		tcm_hcd->watchdog.count = 0;
		return;
	}

	if (en) {
		queue_delayed_work(tcm_hcd->watchdog.workqueue,	&tcm_hcd->watchdog.work,
			msecs_to_jiffies(WATCHDOG_DELAY_MS));
	} else {
		tcm_hcd->watchdog.count = 0;
	}

	return;
}

static void syna_tcm_watchdog_work(struct work_struct *work)
{
	int retval;
	unsigned char marker;
	struct delayed_work *delayed_work = container_of(work, struct delayed_work, work);
	struct syna_tcm_watchdog *watchdog =
								container_of(delayed_work, struct syna_tcm_watchdog, work);
	struct syna_tcm_hcd *tcm_hcd = container_of(watchdog, struct syna_tcm_hcd, watchdog);

	if (mutex_is_locked(&tcm_hcd->rw_ctrl_mutex))
		goto exit;

	mutex_lock(&tcm_hcd->rw_ctrl_mutex);

	retval = syna_tcm_read(tcm_hcd, &marker, 1);

	mutex_unlock(&tcm_hcd->rw_ctrl_mutex);

	if (retval < 0 || marker != MESSAGE_MARKER) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to read from device\n");

		tcm_hcd->watchdog.count++;

		if (tcm_hcd->watchdog.count >= WATCHDOG_TRIGGER_COUNT) {
			retval = tcm_hcd->reset_n_reinit(tcm_hcd, true, false);
			if (retval < 0) {
				input_err(true, tcm_hcd->pdev->dev.parent, "Failed to do reset and reinit\n");
			}
			tcm_hcd->watchdog.count = 0;
		}
	}

exit:
	queue_delayed_work(tcm_hcd->watchdog.workqueue, &tcm_hcd->watchdog.work,
			msecs_to_jiffies(WATCHDOG_DELAY_MS));

	return;
}
#endif

static void syna_tcm_polling_work(struct work_struct *work)
{
	int retval;
	struct delayed_work *delayed_work = container_of(work, struct delayed_work, work);
	struct syna_tcm_hcd *tcm_hcd =
						container_of(delayed_work, struct syna_tcm_hcd, polling_work);

	if (!tcm_hcd->do_polling)
		return;

	retval = tcm_hcd->read_message(tcm_hcd, NULL, 0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to read message\n");
		if (retval == -ENXIO && tcm_hcd->hw_if->bus_io->type == BUS_SPI)
			syna_tcm_check_hdl(tcm_hcd, REPORT_HDL_F35);
	}

	if ((tcm_hcd->lp_state == PWR_ON) || (retval >= 0)) {
		queue_delayed_work(tcm_hcd->polling_workqueue, &tcm_hcd->polling_work,
				msecs_to_jiffies(POLLING_DELAY_MS));
	}

	return;
}

static irqreturn_t syna_tcm_isr(int irq, void *data)
{
	int retval;
	unsigned int loop;
	unsigned int irq_n;
	int attn_before;
	int attn_after;
	struct syna_tcm_hcd *tcm_hcd = data;
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	if (unlikely(gpio_get_value(bdata->irq_gpio) != bdata->irq_on_state))
		goto exit;

	tcm_hcd->isr_pid = current->pid;
	since58_irq_cnt++;
	irq_n = since58_irq_cnt;

	for (loop = 0; loop < SAAIOS_IRQ_DRAIN_MAX; loop++) {
		attn_before = gpio_get_value(bdata->irq_gpio);
		if (attn_before != bdata->irq_on_state)
			break;

		retval = tcm_hcd->read_message(tcm_hcd, NULL, 0);
		attn_after = gpio_get_value(bdata->irq_gpio);

		if (saaios_irq_detail_n < SAAIOS_IRQ_DETAIL_MAX) {
			unsigned char raw[8];
			unsigned int i;

			memset(raw, 0, sizeof(raw));
			LOCK_BUFFER(tcm_hcd->in);
			if (tcm_hcd->in.buf) {
				for (i = 0; i < 8 && i < tcm_hcd->in.buf_size; i++)
					raw[i] = tcm_hcd->in.buf[i];
			}
			UNLOCK_BUFFER(tcm_hcd->in);
			pr_info("SAaiOS_TOUCH_DBG: IRQ_LOOP irq_n=%u loop=%u attn_before=%d attn_after=%d irq_on=%d read_retval=%d raw=%02x %02x %02x %02x %02x %02x %02x %02x read_length=%u report=0x%02x plen=%u mode=0x%02x host_downloading=%d cmd=0x%02x\n",
				irq_n, loop, attn_before, attn_after,
				bdata->irq_on_state, retval,
				raw[0], raw[1], raw[2], raw[3],
				raw[4], raw[5], raw[6], raw[7],
				tcm_hcd->read_length, tcm_hcd->status_report_code,
				tcm_hcd->payload_length, tcm_hcd->id_info.mode,
				atomic_read(&tcm_hcd->host_downloading),
				tcm_hcd->command);
			saaios_irq_detail_n++;
		}

		if (retval < 0) {
			if (tcm_hcd->sensor_type == TYPE_F35)
				syna_tcm_check_hdl(tcm_hcd, REPORT_HDL_F35);
			else
				input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to read message\n");
			break;
		}
		if (atomic_read(&saaios_irq_held))
			break;
	}

	if (loop >= SAAIOS_IRQ_DRAIN_MAX &&
			gpio_get_value(bdata->irq_gpio) == bdata->irq_on_state)
		saaios_irq_emergency_hold(tcm_hcd,
			"32 messages one IRQ entry, ATTN still asserted");

exit:
	return IRQ_HANDLED;
}

static int syna_tcm_enable_irq(struct syna_tcm_hcd *tcm_hcd, bool en, bool ns)
{
	int retval;
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;
	static bool irq_freed = true;

	mutex_lock(&tcm_hcd->irq_en_mutex);

	if (en) {
		if (tcm_hcd->irq_enabled) {
			input_info(true, tcm_hcd->pdev->dev.parent, "Interrupt already enabled\n");
			retval = 0;
			goto exit;
		}

		if (bdata->irq_gpio < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Invalid IRQ GPIO\n");
			retval = -EINVAL;
			goto queue_polling_work;
		}

		if (irq_freed) {
			retval = request_threaded_irq(tcm_hcd->irq, NULL, syna_tcm_isr, bdata->irq_flags,
							PLATFORM_DRIVER_NAME, tcm_hcd);
			if (retval < 0) {
				input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to create interrupt thread\n");
			}
		} else {
			enable_irq(tcm_hcd->irq);
			retval = 0;
		}

queue_polling_work:
		if (retval < 0) {
#ifdef FALL_BACK_ON_POLLING
			queue_delayed_work(tcm_hcd->polling_workqueue, &tcm_hcd->polling_work,
				msecs_to_jiffies(POLLING_DELAY_MS));
			tcm_hcd->do_polling = true;
			retval = 0;
#endif
		}

		if (retval < 0)
			goto exit;
		else
			msleep(ENABLE_IRQ_DELAY_MS);
	} else {
		if (!tcm_hcd->irq_enabled) {
			input_dbg(true, tcm_hcd->pdev->dev.parent, "Interrupt already disabled\n");
			retval = 0;
			goto exit;
		}

		if (bdata->irq_gpio >= 0) {
			if (ns) {
				disable_irq_nosync(tcm_hcd->irq);
			} else {
				disable_irq(tcm_hcd->irq);
				free_irq(tcm_hcd->irq, tcm_hcd);
			}
			irq_freed = !ns;
		}

		if (ns) {
			cancel_delayed_work(&tcm_hcd->polling_work);
		} else {
			cancel_delayed_work_sync(&tcm_hcd->polling_work);
			flush_workqueue(tcm_hcd->polling_workqueue);
		}

		tcm_hcd->do_polling = false;
	}

	retval = 0;

exit:
	if (retval == 0)
		tcm_hcd->irq_enabled = en;

	mutex_unlock(&tcm_hcd->irq_en_mutex);

	return retval;
}

static int syna_tcm_set_gpio(struct syna_tcm_hcd *tcm_hcd, int gpio,
		bool config, int dir, int state)
{
	int retval;
	char label[16];

	if (config) {
		retval = snprintf(label, 16, "tcm_gpio_%d\n", gpio);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to set GPIO label\n");
			return retval;
		}

		retval = gpio_request(gpio, label);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to request GPIO %d\n", gpio);
			return retval;
		}

		if (dir == 0)
			retval = gpio_direction_input(gpio);
		else
			retval = gpio_direction_output(gpio, state);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to set GPIO %d direction\n", gpio);
			return retval;
		}
	} else {
		gpio_free(gpio);
	}

	return 0;
}

static int syna_tcm_config_gpio(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	if (bdata->irq_gpio >= 0) {
		retval = syna_tcm_set_gpio(tcm_hcd, bdata->irq_gpio, true, 0, 0);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to configure interrupt GPIO\n");
			goto err_set_gpio_irq;
		}
	}

	if (bdata->cs_gpio >= 0) {
		retval = syna_tcm_set_gpio(tcm_hcd, bdata->cs_gpio, true, 1, 0);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to configure interrupt GPIO\n");
			goto err_set_gpio_cs;
		}
	}

	if (bdata->power_gpio >= 0) {
		retval = syna_tcm_set_gpio(tcm_hcd, bdata->power_gpio, true, 1, !bdata->power_on_state);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to configure power GPIO\n");
			goto err_set_gpio_power;
		}
	}

	if (bdata->reset_gpio >= 0) {
		retval = syna_tcm_set_gpio(tcm_hcd, bdata->reset_gpio, true, 1, !bdata->reset_on_state);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to configure reset GPIO\n");
			goto err_set_gpio_reset;
		}
	}

	if (bdata->power_gpio >= 0) {
		gpio_set_value(bdata->power_gpio, bdata->power_on_state);
		msleep(bdata->power_delay_ms);
	}

	if (bdata->reset_gpio >= 0) {
		gpio_set_value(bdata->reset_gpio, bdata->reset_on_state);
		msleep(bdata->reset_active_ms);
		gpio_set_value(bdata->reset_gpio, !bdata->reset_on_state);
		msleep(bdata->reset_delay_ms);
	}

	return 0;

err_set_gpio_reset:
	if (bdata->power_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->power_gpio, false, 0, 0);
err_set_gpio_power:
	if (bdata->irq_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->cs_gpio, false, 0, 0);
err_set_gpio_cs:
	if (bdata->irq_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->irq_gpio, false, 0, 0);
err_set_gpio_irq:
	return retval;
}

static int syna_tcm_enable_regulator(struct syna_tcm_hcd *tcm_hcd, bool en)
{
	int retval;
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	if (!en) {
		retval = 0;
		goto disable_pwr_reg;
	}

	if (tcm_hcd->bus_reg) {
		retval = regulator_enable(tcm_hcd->bus_reg);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to enable bus regulator\n");
			goto exit;
		}
	}

	if (tcm_hcd->pwr_reg) {
		retval = regulator_enable(tcm_hcd->pwr_reg);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to enable power regulator\n");
			goto disable_bus_reg;
		}
		msleep(bdata->power_delay_ms);
	}

	return 0;

disable_pwr_reg:
	if (tcm_hcd->pwr_reg)
		regulator_disable(tcm_hcd->pwr_reg);

disable_bus_reg:
	if (tcm_hcd->bus_reg)
		regulator_disable(tcm_hcd->bus_reg);

exit:
	return retval;
}

static int syna_tcm_get_regulator(struct syna_tcm_hcd *tcm_hcd, bool get)
{
	int retval;
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	if (!get) {
		retval = 0;
		goto regulator_put;
	}

	if (bdata->bus_reg_name != NULL && *bdata->bus_reg_name != 0) {
		tcm_hcd->bus_reg = regulator_get(tcm_hcd->pdev->dev.parent, bdata->bus_reg_name);
		if (IS_ERR(tcm_hcd->bus_reg)) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to get bus regulator\n");
			retval = PTR_ERR(tcm_hcd->bus_reg);
			goto regulator_put;
		}
	}

	if (bdata->pwr_reg_name != NULL && *bdata->pwr_reg_name != 0) {
		tcm_hcd->pwr_reg = regulator_get(tcm_hcd->pdev->dev.parent, bdata->pwr_reg_name);
		if (IS_ERR(tcm_hcd->pwr_reg)) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to get power regulator\n");
			retval = PTR_ERR(tcm_hcd->pwr_reg);
			goto regulator_put;
		}
	}

	return 0;

regulator_put:
	if (tcm_hcd->bus_reg) {
		regulator_put(tcm_hcd->bus_reg);
		tcm_hcd->bus_reg = NULL;
	}

	if (tcm_hcd->pwr_reg) {
		regulator_put(tcm_hcd->pwr_reg);
		tcm_hcd->pwr_reg = NULL;
	}

	return retval;
}

int syna_tcm_get_app_info(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;
	unsigned int timeout;

	timeout = APP_STATUS_POLL_TIMEOUT_MS;

	resp_buf = NULL;
	resp_buf_size = 0;

get_app_info:
	pr_info("SAaiOS_TOUCH_DBG: 0x20 GET_APPLICATION_INFO start mode=0x%02x\n",
		tcm_hcd->id_info.mode);
	retval = tcm_hcd->write_message(tcm_hcd, CMD_GET_APPLICATION_INFO,
				NULL, 0, &resp_buf,	&resp_buf_size, &resp_length, NULL, 0);
	if (retval < 0) {
		pr_info("SAaiOS_TOUCH_DBG: 0x20 GET_APPLICATION_INFO retval=%d timeout=%d mode=0x%02x\n",
			retval, retval == -ETIME ? 1 : 0, tcm_hcd->id_info.mode);
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_GET_APPLICATION_INFO));
		goto exit;
	}

	retval = secure_memcpy((unsigned char *)&tcm_hcd->app_info,
				sizeof(tcm_hcd->app_info), resp_buf, resp_buf_size,
				MIN(sizeof(tcm_hcd->app_info), resp_length));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy application info\n");
		goto exit;
	}

	tcm_hcd->app_status = le2_to_uint(tcm_hcd->app_info.status);
	pr_info("SAaiOS_TOUCH_DBG: 0x20 GET_APPLICATION_INFO retval=0 app_status=%s (0x%04x) resp_len=%u mode=0x%02x\n",
		tcm_hcd->app_status == APP_STATUS_OK ? "OK" :
		tcm_hcd->app_status == APP_STATUS_BOOTING ? "BOOTING" :
		tcm_hcd->app_status == APP_STATUS_UPDATING ? "UPDATING" : "other",
		tcm_hcd->app_status, resp_length, tcm_hcd->id_info.mode);

	if (tcm_hcd->app_status == APP_STATUS_BOOTING ||
				tcm_hcd->app_status == APP_STATUS_UPDATING) {
		if (timeout > 0) {
			msleep(APP_STATUS_POLL_MS);
			timeout -= APP_STATUS_POLL_MS;
			goto get_app_info;
		}
	}

	input_info(true, tcm_hcd->pdev->dev.parent, "config version %02X%02X%02X%02X\n",
		tcm_hcd->app_info.customer_config_id[0], tcm_hcd->app_info.customer_config_id[1],
		tcm_hcd->app_info.customer_config_id[2], tcm_hcd->app_info.customer_config_id[3]);

	tcm_hcd->cols = le2_to_uint(tcm_hcd->app_info.num_of_image_cols);
	tcm_hcd->rows = le2_to_uint(tcm_hcd->app_info.num_of_image_rows);

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}

static int syna_tcm_get_boot_info(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;

	resp_buf = NULL;
	resp_buf_size = 0;

	retval = tcm_hcd->write_message(tcm_hcd, CMD_GET_BOOT_INFO, NULL,
				0, &resp_buf, &resp_buf_size, &resp_length, NULL, 0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to write command %s\n", STR(CMD_GET_BOOT_INFO));
		goto exit;
	}

	retval = secure_memcpy((unsigned char *)&tcm_hcd->boot_info,
				sizeof(tcm_hcd->boot_info), resp_buf, resp_buf_size,
				MIN(sizeof(tcm_hcd->boot_info), resp_length));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy boot info\n");
		goto exit;
	}

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}

static int syna_tcm_get_romboot_info(struct syna_tcm_hcd *tcm_hcd)
{
	/* since59: 0x40 alone switches this IC to on-chip HDL 0x02.
	 * Never send it. since76: delayed oneshot 0x45; skip switch_mode
	 * when already HDL 0x02; leftover HELP_SEND_REINIT when mode is
	 * 0x01 or 0x02 and host_downloading=0 (identify(false)/0x20 only;
	 * do not touch_reinit/0x25 — 0x25 -5 may desync SPI before 0x30);
	 * no auto 0x05/0x23/0x26; oneshot 0x30 from sysfs. No forced APP_CONFIG.
	 */
	pr_info("SAaiOS_TOUCH_DBG: skip CMD_GET_ROMBOOT_INFO 0x40 (would leave RomBoot 0x04)\n");
	return 0;
}

static int syna_tcm_identify(struct syna_tcm_hcd *tcm_hcd, bool id)
{
	int retval;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;
	unsigned int max_write_size;

	resp_buf = NULL;
	resp_buf_size = 0;

	mutex_lock(&tcm_hcd->identify_mutex);

	if (!id)
		goto get_info;

	retval = tcm_hcd->write_message(tcm_hcd, CMD_IDENTIFY,
				NULL, 0, &resp_buf, &resp_buf_size, &resp_length, NULL, 0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_IDENTIFY));
		goto exit;
	}

	retval = secure_memcpy((unsigned char *)&tcm_hcd->id_info,
				sizeof(tcm_hcd->id_info), resp_buf, resp_buf_size,
				MIN(sizeof(tcm_hcd->id_info), resp_length));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy identification info\n");
		goto exit;
	}

	tcm_hcd->packrat_number = le4_to_uint(tcm_hcd->id_info.build_id);

	max_write_size = le2_to_uint(tcm_hcd->id_info.max_write_size);
	tcm_hcd->wr_chunk_size = MIN(max_write_size, WR_CHUNK_SIZE);
	if (tcm_hcd->wr_chunk_size == 0)
		tcm_hcd->wr_chunk_size = max_write_size;

	input_info(true, tcm_hcd->pdev->dev.parent,
		"Firmware build id = %d\n", tcm_hcd->packrat_number);

get_info:
	switch (tcm_hcd->id_info.mode) {
	case MODE_APPLICATION_FIRMWARE:
	case MODE_HOSTDOWNLOAD_FIRMWARE:

		retval = syna_tcm_get_app_info(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to get application info\n");
			goto exit;
		}
		break;
	case MODE_BOOTLOADER:
	case MODE_TDDI_BOOTLOADER:

		input_dbg(true, tcm_hcd->pdev->dev.parent, "In bootloader mode\n");

		retval = syna_tcm_get_boot_info(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to get boot info\n");
			goto exit;
		}
		break;
	case MODE_ROMBOOTLOADER:

		input_dbg(true, tcm_hcd->pdev->dev.parent, "In rombootloader mode\n");

		retval = syna_tcm_get_romboot_info(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to get application info\n");
			goto exit;
		}
		break;
	default:
		break;
	}

	retval = 0;

exit:
	mutex_unlock(&tcm_hcd->identify_mutex);

	kfree(resp_buf);

	return retval;
}

static int syna_tcm_run_production_test_firmware(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	bool retry;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;

	retry = true;

	resp_buf = NULL;
	resp_buf_size = 0;

retry:
	retval = tcm_hcd->write_message(tcm_hcd, CMD_ENTER_PRODUCTION_TEST_MODE,
				NULL, 0, &resp_buf, &resp_buf_size, &resp_length,
				NULL, MODE_SWITCH_DELAY_MS);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_ENTER_PRODUCTION_TEST_MODE));
		goto exit;
	}

	if (tcm_hcd->id_info.mode != MODE_PRODUCTIONTEST_FIRMWARE) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to run production test firmware\n");
		if (retry) {
			retry = false;
			goto retry;
		}
		retval = -EINVAL;
		goto exit;
	} else if (tcm_hcd->app_status != APP_STATUS_OK) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Application status = 0x%02x\n", tcm_hcd->app_status);
	}

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}

static int syna_tcm_run_application_firmware(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	bool retry;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;

	retry = true;

	resp_buf = NULL;
	resp_buf_size = 0;

retry:
	retval = tcm_hcd->write_message(tcm_hcd, CMD_RUN_APPLICATION_FIRMWARE,
				NULL, 0, &resp_buf, &resp_buf_size, &resp_length,
				NULL, MODE_SWITCH_DELAY_MS);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_RUN_APPLICATION_FIRMWARE));
		goto exit;
	}

	retval = tcm_hcd->identify(tcm_hcd, false);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to do identification\n");
		goto exit;
	}

	if (IS_NOT_FW_MODE(tcm_hcd->id_info.mode)) {
		input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to run application firmware (boot status = 0x%02x)\n",
				tcm_hcd->boot_info.status);
		if (retry) {
			retry = false;
			goto retry;
		}
		retval = -EINVAL;
		goto exit;
	} else if (tcm_hcd->app_status != APP_STATUS_OK) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Application status = 0x%02x\n", tcm_hcd->app_status);
	}

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}

static int syna_tcm_run_bootloader_firmware(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;
	unsigned char command;

	resp_buf = NULL;
	resp_buf_size = 0;
	command = (tcm_hcd->id_info.mode == MODE_ROMBOOTLOADER) ?
				CMD_ROMBOOT_RUN_BOOTLOADER_FIRMWARE : CMD_RUN_BOOTLOADER_FIRMWARE;

	pr_info("SAaiOS_TOUCH_DBG: stock run_bootloader_firmware cmd=0x%02x mode=0x%02x part='%s' packrat=%u (0x42 if RomBoot 0x04, 0x1f otherwise)\n",
		command, tcm_hcd->id_info.mode, tcm_hcd->id_info.part_number,
		tcm_hcd->packrat_number);

	retval = tcm_hcd->write_message(tcm_hcd, command,
				NULL, 0, &resp_buf, &resp_buf_size,
				&resp_length, NULL, MODE_SWITCH_DELAY_MS);
	pr_info("SAaiOS_TOUCH_DBG: stock run_bootloader_firmware write_message cmd=0x%02x retval=%d mode=0x%02x part='%s' packrat=%u\n",
		command, retval, tcm_hcd->id_info.mode,
		tcm_hcd->id_info.part_number, tcm_hcd->packrat_number);
	if (retval < 0) {
		if (tcm_hcd->id_info.mode == MODE_ROMBOOTLOADER) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to write command %s\n", STR(CMD_ROMBOOT_RUN_BOOTLOADER_FIRMWARE));
		} else {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to write command %s\n", STR(CMD_RUN_BOOTLOADER_FIRMWARE));
		}
		goto exit;
	}

	if (command != CMD_ROMBOOT_RUN_BOOTLOADER_FIRMWARE) {
		retval = tcm_hcd->identify(tcm_hcd, false);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to do identification\n");
		goto exit;
		}

		if (IS_FW_MODE(tcm_hcd->id_info.mode)) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to enter bootloader mode\n");
			retval = -EINVAL;
			goto exit;
		}
	}

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}

static int syna_tcm_switch_mode(struct syna_tcm_hcd *tcm_hcd,
		enum firmware_mode mode)
{
	int retval;

	mutex_lock(&tcm_hcd->reset_mutex);

#ifdef WATCHDOG_SW
	tcm_hcd->update_watchdog(tcm_hcd, false);
#endif

	switch (mode) {
	case FW_MODE_BOOTLOADER:
		retval = syna_tcm_run_bootloader_firmware(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to switch to bootloader mode\n");
			goto exit;
		}
		break;
	case FW_MODE_APPLICATION:
		retval = syna_tcm_run_application_firmware(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to switch to application mode\n");
			goto exit;
		}
		break;
	case FW_MODE_PRODUCTION_TEST:
		retval = syna_tcm_run_production_test_firmware(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to switch to production test mode\n");
			goto exit;
		}
		break;
	default:
		input_err(true, tcm_hcd->pdev->dev.parent, "Invalid firmware mode\n");
		retval = -EINVAL;
		goto exit;
	}

	retval = 0;

exit:
#ifdef WATCHDOG_SW
	tcm_hcd->update_watchdog(tcm_hcd, true);
#endif

	mutex_unlock(&tcm_hcd->reset_mutex);

	return retval;
}

static int syna_tcm_get_dynamic_config(struct syna_tcm_hcd *tcm_hcd,
		enum dynamic_config_id id, unsigned short *value)
{
	int retval;
	unsigned char out_buf;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;

	resp_buf = NULL;
	resp_buf_size = 0;

	out_buf = (unsigned char)id;

	retval = tcm_hcd->write_message(tcm_hcd, CMD_GET_DYNAMIC_CONFIG,
				&out_buf, sizeof(out_buf), &resp_buf,
				&resp_buf_size, &resp_length, NULL, 0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_GET_DYNAMIC_CONFIG));
		goto exit;
	}

	if (resp_length < 2) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Invalid data length\n");
		retval = -EINVAL;
		goto exit;
	}

	*value = (unsigned short)le2_to_uint(resp_buf);

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}

static int syna_tcm_set_dynamic_config(struct syna_tcm_hcd *tcm_hcd,
		enum dynamic_config_id id, unsigned short value)
{
	int retval;
	unsigned char out_buf[3];
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;

	resp_buf = NULL;
	resp_buf_size = 0;

	input_info(true, tcm_hcd->pdev->dev.parent,
			"set dynamic cmd %s  id:%x,  value:%d\n", STR(CMD_SET_DYNAMIC_CONFIG), id, value);

	out_buf[0] = (unsigned char)id;
	out_buf[1] = (unsigned char)value;
	out_buf[2] = (unsigned char)(value >> 8);

	retval = tcm_hcd->write_message(tcm_hcd, CMD_SET_DYNAMIC_CONFIG,
				out_buf, sizeof(out_buf), &resp_buf,
				&resp_buf_size, &resp_length, NULL, 0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_SET_DYNAMIC_CONFIG));
		goto exit;
	}

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}
int syna_tcm_set_scan_start_stop_cmd(struct syna_tcm_hcd *tcm_hcd, unsigned char value)
{
	int retval;
	unsigned char out_buf;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;

	resp_buf = NULL;
	resp_buf_size = 0;
	
	out_buf = 0x11;
	if (value == 1) {
		out_buf = 0x11;
	} else if (value == 0) {
		out_buf = 0x10;
	}

	input_err(true, tcm_hcd->pdev->dev.parent,
			"set start stop cmd B0 %s value:%d\n", STR(CMD_SET_SCAN_START_STOP), out_buf);	

	retval = tcm_hcd->write_message(tcm_hcd, CMD_SET_SCAN_START_STOP,
				&out_buf, sizeof(out_buf), &resp_buf,
				&resp_buf_size, &resp_length, NULL, 0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_SET_SCAN_START_STOP));
		goto exit;
	}

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}

static int syna_tcm_get_data_location(struct syna_tcm_hcd *tcm_hcd,
		enum flash_area area, unsigned int *addr, unsigned int *length)
{
	int retval;
	unsigned char out_buf;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;

	switch (area) {
	case CUSTOM_LCM:
		out_buf = LCM_DATA;
		break;
	case CUSTOM_OEM:
		out_buf = OEM_DATA;
		break;
	case PPDT:
		out_buf = PPDT_DATA;
		break;
	default:
		input_err(true, tcm_hcd->pdev->dev.parent, "Invalid flash area\n");
		return -EINVAL;
	}

	resp_buf = NULL;
	resp_buf_size = 0;

	retval = tcm_hcd->write_message(tcm_hcd, CMD_GET_DATA_LOCATION,
				&out_buf, sizeof(out_buf), &resp_buf,
				&resp_buf_size, &resp_length, NULL, 0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_GET_DATA_LOCATION));
		goto exit;
	}

	if (resp_length != 4) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Invalid data length\n");
		retval = -EINVAL;
		goto exit;
	}

	*addr = le2_to_uint(&resp_buf[0]);
	*length = le2_to_uint(&resp_buf[2]);

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}

static int syna_tcm_sleep(struct syna_tcm_hcd *tcm_hcd, bool en)
{
	int retval;
	unsigned char command;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;

	command = en ? CMD_ENTER_DEEP_SLEEP : CMD_EXIT_DEEP_SLEEP;

	resp_buf = NULL;
	resp_buf_size = 0;

	retval = tcm_hcd->write_message(tcm_hcd, command,
				NULL, 0, &resp_buf, &resp_buf_size,
				&resp_length, NULL, 0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n",
			en ? STR(CMD_ENTER_DEEP_SLEEP) : STR(CMD_EXIT_DEEP_SLEEP));
		goto exit;
	}

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}

static int syna_tcm_reset(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	retval = tcm_hcd->write_message(tcm_hcd, CMD_RESET,
				NULL, 0, &resp_buf, &resp_buf_size,
				&resp_length, NULL, bdata->reset_delay_ms);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_RESET));
	}

	return retval;
}

static int syna_tcm_reset_and_reinit(struct syna_tcm_hcd *tcm_hcd,
		bool hw, bool update_wd)
{
	int retval;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;
	struct syna_tcm_module_handler *mod_handler;
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	resp_buf = NULL;
	resp_buf_size = 0;

	mutex_lock(&tcm_hcd->reset_mutex);

#ifdef WATCHDOG_SW
	if (update_wd)
		tcm_hcd->update_watchdog(tcm_hcd, false);
#endif

	if (hw) {
		if (bdata->reset_gpio < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Hardware reset unavailable\n");
			retval = -EINVAL;
			goto exit;
		}
		gpio_set_value(bdata->reset_gpio, bdata->reset_on_state);
		msleep(bdata->reset_active_ms);
		gpio_set_value(bdata->reset_gpio, !bdata->reset_on_state);
	} else {
		retval = syna_tcm_reset(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to do reset\n");
			goto exit;
		}
	}

	/* for hdl, the remaining re-init process will be done */
	/* in the helper thread, so wait for the completion here */
	if (tcm_hcd->in_hdl_mode) {
		mutex_unlock(&tcm_hcd->reset_mutex);
		kfree(resp_buf);

		retval = syna_tcm_wait_hdl(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to wait for completion of host download\n");
			return retval;
		}

#ifdef WATCHDOG_SW
		if (update_wd)
			tcm_hcd->update_watchdog(tcm_hcd, true);
#endif
		return 0;
	}

	msleep(bdata->reset_delay_ms);

	retval = tcm_hcd->identify(tcm_hcd, false);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to do identification\n");
		goto exit;
	}

	if (IS_FW_MODE(tcm_hcd->id_info.mode))
		goto get_features;

	retval = tcm_hcd->write_message(tcm_hcd, CMD_RUN_APPLICATION_FIRMWARE,
				NULL, 0, &resp_buf, &resp_buf_size,
				&resp_length, NULL, MODE_SWITCH_DELAY_MS);
	if (retval < 0) {
		input_info(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_RUN_APPLICATION_FIRMWARE));
	}

	retval = tcm_hcd->identify(tcm_hcd, false);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to do identification\n");
		goto exit;
	}

get_features:
	input_info(true, tcm_hcd->pdev->dev.parent, 
		"Firmware mode = 0x%02x\n", tcm_hcd->id_info.mode);

	if (IS_NOT_FW_MODE(tcm_hcd->id_info.mode)) {
		input_info(true, tcm_hcd->pdev->dev.parent, 
			"Boot status = 0x%02x\n", tcm_hcd->boot_info.status);
	} else if (tcm_hcd->app_status != APP_STATUS_OK) {
		input_info(true, tcm_hcd->pdev->dev.parent,
			"Application status = 0x%02x\n", tcm_hcd->app_status);
	}

	if (IS_NOT_FW_MODE(tcm_hcd->id_info.mode))
		goto dispatch_reinit;

	retval = tcm_hcd->write_message(tcm_hcd, CMD_GET_FEATURES,
				NULL, 0, &resp_buf, &resp_buf_size,
				&resp_length, NULL, 0);
	if (retval < 0) {
		input_info(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_GET_FEATURES));
	} else {
		retval = secure_memcpy((unsigned char *)&tcm_hcd->features,
					sizeof(tcm_hcd->features), resp_buf, resp_buf_size,
					MIN(sizeof(tcm_hcd->features), resp_length));
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy feature description\n");
		}
	}

dispatch_reinit:
	mutex_lock(&mod_pool.mutex);

	if (!list_empty(&mod_pool.list)) {
		list_for_each_entry(mod_handler, &mod_pool.list, link) {
			if (!mod_handler->insert && !mod_handler->detach && (mod_handler->mod_cb->reinit))
				mod_handler->mod_cb->reinit(tcm_hcd);
		}
	}

	mutex_unlock(&mod_pool.mutex);

	retval = 0;

exit:
#ifdef WATCHDOG_SW
	if (update_wd)
		tcm_hcd->update_watchdog(tcm_hcd, true);
#endif

	mutex_unlock(&tcm_hcd->reset_mutex);

	kfree(resp_buf);

	return retval;
}

#ifdef USE_FLASH
static int syna_tcm_rezero(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	unsigned char *resp_buf;
	unsigned int resp_buf_size;
	unsigned int resp_length;

	resp_buf = NULL;
	resp_buf_size = 0;

	retval = tcm_hcd->write_message(tcm_hcd, CMD_REZERO,
				NULL, 0, &resp_buf, &resp_buf_size,
				&resp_length, NULL, 0);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to write command %s\n", STR(CMD_REZERO));
		goto exit;
	}

	retval = 0;

exit:
	kfree(resp_buf);

	return retval;
}
#endif

/*
 * since70 (falsified): one-shot late module resume after post-HDL
 * touch_reinit. Ran, sent no SPI, IRQ stayed 4. Kept after 0x05 so
 * sensing-enable order is enable-report then resume.
 * This Samsung OSS does not register touch as a TCM module (.resume is
 * NULL on every mod_cb). syna_tcm_resume's "mod_resume" label is NOT a
 * callback — it is inline PM. The stock named touch resume is
 * touch_resume() (was #if 0). Dispatch mod_cb->resume then touch_resume.
 * Do not call full syna_tcm_resume (panel / 0x45 / switch_mode / wait_hdl).
 * Gate uses IS_FW_MODE (0x01 or 0x02). MODE_APPLICATION_FIRMWARE is 0x01;
 * live HDL after 0x45 is MODE_HOSTDOWNLOAD_FIRMWARE 0x02.
 */
static bool late_touch_resumed;

static int syna_tcm_mod_resume(struct syna_tcm_hcd *tcm_hcd)
{
	int retval = 0;
	int n = 0;
	struct syna_tcm_module_handler *mod_handler;

	mutex_lock(&mod_pool.mutex);
	if (!list_empty(&mod_pool.list)) {
		list_for_each_entry(mod_handler, &mod_pool.list, link) {
			if (!mod_handler->insert && !mod_handler->detach &&
					mod_handler->mod_cb &&
					mod_handler->mod_cb->resume) {
				retval = mod_handler->mod_cb->resume(tcm_hcd);
				n++;
			}
		}
	}
	mutex_unlock(&mod_pool.mutex);
	pr_info("SAaiOS_TOUCH_DBG: late touch resume mod_cb->resume n=%d last_retval=%d\n",
		n, retval);

	retval = touch_resume(tcm_hcd);
	pr_info("SAaiOS_TOUCH_DBG: late touch resume touch_resume retval=%d\n",
		retval);
	return retval;
}

static void __maybe_unused syna_tcm_late_touch_resume(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	int hdl;
	int init_ok;

	if (!tcm_hcd || late_touch_resumed)
		return;

	hdl = atomic_read(&tcm_hcd->host_downloading);
	init_ok = touch_init_ok();

	/* Do not require MODE_APPLICATION_FIRMWARE (0x01). Live mode is 0x02. */
	if (!init_ok || !IS_FW_MODE(tcm_hcd->id_info.mode) || hdl ||
			!tcm_hcd->fb_ready) {
		pr_info("SAaiOS_TOUCH_DBG: late touch resume skip init_touch_ok=%d mode=0x%02x host_downloading=%d fb_ready=%u\n",
			init_ok, tcm_hcd->id_info.mode, hdl, tcm_hcd->fb_ready);
		return;
	}

	late_touch_resumed = true;
	pr_info("SAaiOS_TOUCH_DBG: late touch resume start: lp_state=%d boot_resume=%d mode=0x%02x fb_ready=%u init_touch_ok=%d host_downloading=%d irq_cnt=%u (IS_FW_MODE 0x01|0x02, call touch_resume)\n",
		tcm_hcd->lp_state, tcm_hcd->boot_resume ? 1 : 0,
		tcm_hcd->id_info.mode, tcm_hcd->fb_ready, init_ok, hdl,
		since58_irq_cnt);
	retval = syna_tcm_mod_resume(tcm_hcd);
	pr_info("SAaiOS_TOUCH_DBG: late touch resume done retval=%d lp_state=%d irq_cnt=%u\n",
		retval, tcm_hcd->lp_state, since58_irq_cnt);
}

static void syna_tcm_helper_work(struct work_struct *work)
{
	int retval;
	int attn;
	int hdl;
	unsigned char task;
	struct syna_tcm_helper *helper = container_of(work, struct syna_tcm_helper, work);
	struct syna_tcm_hcd *tcm_hcd = container_of(helper, struct syna_tcm_hcd, helper);

	task = atomic_read(&helper->task);

	switch (task) {

	/* this helper can help to run the application firmware */
	case HELP_RUN_APPLICATION_FIRMWARE:
		mutex_lock(&tcm_hcd->reset_mutex);

#ifdef WATCHDOG_SW
		tcm_hcd->update_watchdog(tcm_hcd, false);
#endif
		retval = syna_tcm_run_application_firmware(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to switch to application mode\n");
		}
#ifdef WATCHDOG_SW
		tcm_hcd->update_watchdog(tcm_hcd, true);
#endif
		mutex_unlock(&tcm_hcd->reset_mutex);
		break;

	/* the reinit helper is used to notify all installed modules to */
	/* do the re-initialization process, since the HDL is completed */
	case HELP_SEND_REINIT_NOTIFICATION:
		attn = -1;
		hdl = atomic_read(&tcm_hcd->host_downloading);
		if (tcm_hcd->hw_if && tcm_hcd->hw_if->bdata &&
				tcm_hcd->hw_if->bdata->irq_gpio >= 0)
			attn = gpio_get_value(tcm_hcd->hw_if->bdata->irq_gpio);
		/* Gate: IS_FW_MODE is MODE_APPLICATION_FIRMWARE 0x01 or
		 * MODE_HOSTDOWNLOAD_FIRMWARE 0x02. Live post-0x45 is 0x02.
		 * There is no MODE_APPLICATION define; 0x01-only never
		 * matches this IC. Defer RomBoot 0x04 or host_downloading=1.
		 */
		if (!IS_FW_MODE(tcm_hcd->id_info.mode) || hdl) {
			pr_info("SAaiOS_TOUCH_DBG: defer REINIT: mode=0x%02x host_downloading=%d ATTN=%d\n",
				tcm_hcd->id_info.mode, hdl, attn);
			wake_up_interruptible(&tcm_hcd->hdl_wq);
			break;
		}
		pr_info("SAaiOS_TOUCH_DBG: HELP_SEND_REINIT enter mode=0x%02x host_downloading=%d ATTN=%d part='%s' packrat=%u\n",
			tcm_hcd->id_info.mode, hdl, attn,
			tcm_hcd->id_info.part_number, tcm_hcd->packrat_number);
		mutex_lock(&tcm_hcd->reset_mutex);
#ifdef WATCHDOG_SW
		tcm_hcd->update_watchdog(tcm_hcd, false);
#endif
		/* Original stock: identify(false) → GET_APPLICATION_INFO 0x20
		 * (no CMD_IDENTIFY 0x02), then touch_reinit → 0x25.
		 * since76 discriminator: 0x25 still -5 after read-floor 256
		 * (padding/EIO). That failed RX sits before the user's 0x30
		 * and may desync SPI so the IC never sees a coherent
		 * DOWNLOAD_CONFIG. Keep 0x20; do not send 0x25.
		 */
		pr_info("SAaiOS_TOUCH_DBG: HELP_SEND_REINIT identify(false)/0x20 start\n");
		retval = tcm_hcd->identify(tcm_hcd, false);
		pr_info("SAaiOS_TOUCH_DBG: HELP_SEND_REINIT identify(false)/0x20 retval=%d app_status=%s (0x%04x) mode=0x%02x\n",
			retval,
			tcm_hcd->app_status == APP_STATUS_OK ? "OK" :
			tcm_hcd->app_status == APP_STATUS_BOOTING ? "BOOTING" :
			tcm_hcd->app_status == APP_STATUS_UPDATING ? "UPDATING" : "other",
			tcm_hcd->app_status, tcm_hcd->id_info.mode);
		if (retval >= 0 && tcm_hcd->app_status == APP_STATUS_OK) {
			saaios_reinit_ok = 1;
			pr_info("SAaiOS_TOUCH_DBG: saaios_reinit_ok=1 (HELP_SEND_REINIT 0x20 retval>=0 app_status=OK; 0x25 not required)\n");
		}
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to do identification\n");
			pr_info("SAaiOS_TOUCH_DBG: HELP_SEND_REINIT identify(false)/0x20 failed, skip touch_reinit/0x25\n");
			saaios_mark_dead(retval, "REINIT 0x20 failed");
		} else if (IS_FW_MODE(tcm_hcd->id_info.mode)) {
			pr_info("SAaiOS_TOUCH_DBG: skip touch_reinit/0x25 after 0x20 OK (0x25 -5 desync discriminator)\n");
			touch_register_fallback_input(tcm_hcd);
		} else {
			pr_info("SAaiOS_TOUCH_DBG: HELP_SEND_REINIT skip touch_reinit (not FW mode 0x%02x)\n",
				tcm_hcd->id_info.mode);
		}
#ifdef WATCHDOG_SW
		tcm_hcd->update_watchdog(tcm_hcd, true);
#endif
		/* Snapshot mode under reset_mutex: an IRQ between unlock and
		 * the ladder-start check could otherwise mutate id_info.mode
		 * out from under this decision.
		 */
		saaios_reinit_fw_mode = IS_FW_MODE(tcm_hcd->id_info.mode);
		mutex_unlock(&tcm_hcd->reset_mutex);
		/* Ladder AFTER unlock: delay=0 under reset_mutex raced the
		 * successful REINIT 0x20 and timed out, aborting 10/100/500.
		 */
		if (saaios_reinit_ok && saaios_reinit_fw_mode)
			saaios_start_live20_ladder(tcm_hcd);
		syna_tcm_since58_observe(tcm_hcd, "after stock REINIT");
		wake_up_interruptible(&tcm_hcd->hdl_wq);
		break;

	/* this helper is used to reinit the touch reporting */
	case HELP_TOUCH_REINIT:
		hdl = atomic_read(&tcm_hcd->host_downloading);
		if (!IS_FW_MODE(tcm_hcd->id_info.mode) || hdl) {
			pr_info("SAaiOS_TOUCH_DBG: defer HELP_TOUCH_REINIT: mode=0x%02x host_downloading=%d\n",
				tcm_hcd->id_info.mode, hdl);
			break;
		}
		retval = touch_reinit(tcm_hcd);
		pr_info("SAaiOS_TOUCH_DBG: HELP_TOUCH_REINIT touch_reinit retval=%d mode=0x%02x\n",
			retval, tcm_hcd->id_info.mode);
		break;

	/* this helper is used to trigger a romboot hdl */
	case HELP_SEND_ROMBOOT_HDL:
		if (tcm_hcd->romboot_download_deferred) {
			pr_info("SAaiOS_TOUCH_DBG: skip HELP_SEND_ROMBOOT_HDL (0x45 deferred until panel)\n");
			break;
		}
		syna_tcm_check_hdl(tcm_hcd, REPORT_HDL_ROMBOOT);
		break;
	default:
		break;
	}

	atomic_set(&helper->task, HELP_NONE);

	return;
}

int syna_tcm_get_lcd_regulator(struct syna_tcm_hcd *tcm_hcd, bool on)
{
	if (on) {
		tcm_hcd->regulator_vdd = regulator_get(NULL, "vdd_ldo28");
		if (IS_ERR(tcm_hcd->regulator_vdd)) {
			input_err(true, tcm_hcd->pdev->dev.parent, "%s: Failed to get %s regulator.\n",
				 __func__, "vdd_ldo28");
			return PTR_ERR(tcm_hcd->regulator_vdd);
		}

		tcm_hcd->regulator_lcd_reset = regulator_get(NULL, "gpio_lcd_rst");
		if (IS_ERR(tcm_hcd->regulator_lcd_reset)) {
			input_err(true, tcm_hcd->pdev->dev.parent, "%s: Failed to get %s regulator.\n",
				 __func__, "gpio_lcd_rst");
			return PTR_ERR(tcm_hcd->regulator_lcd_reset);
		}

		tcm_hcd->regulator_lcd_bl_en = regulator_get(NULL, "gpio_lcd_bl_en");
		if (IS_ERR(tcm_hcd->regulator_lcd_bl_en)) {
			input_err(true, tcm_hcd->pdev->dev.parent, "%s: Failed to get %s regulator.\n",
				 __func__, "gpio_lcd_bl_en");
			return PTR_ERR(tcm_hcd->regulator_lcd_bl_en);
		}
	} else {
		regulator_put(tcm_hcd->regulator_vdd);
		regulator_put(tcm_hcd->regulator_lcd_reset);
		regulator_put(tcm_hcd->regulator_lcd_bl_en);
	}
	return 0;
}

int syna_tcm_lcd_power_ctrl(struct syna_tcm_hcd *tcm_hcd, bool on)
{
	int retval;
	static bool enabled;

	if (enabled == on) {
		input_err(true, tcm_hcd->pdev->dev.parent, "%s: skip: (%d/%d)\n", __func__, enabled, on);
		return 0;
	}

	if (on) {
		retval = regulator_enable(tcm_hcd->regulator_vdd);
		if (retval) {
			input_err(true, tcm_hcd->pdev->dev.parent, "%s: Failed to enable regulator_vdd: %d\n", __func__, retval);
			return retval;
		}
		retval = regulator_enable(tcm_hcd->regulator_lcd_bl_en);
		if (retval) {
			input_err(true, tcm_hcd->pdev->dev.parent, "%s: Failed to enable regulator_lcd_bl_en: %d\n", __func__, retval);
			return retval;
		}
	} else {
		regulator_disable(tcm_hcd->regulator_vdd);
		regulator_disable(tcm_hcd->regulator_lcd_bl_en);
	}

	enabled = on;

	input_info(true, tcm_hcd->pdev->dev.parent, "%s %d done\n", __func__, on);

	return 0;
}
int syna_tcm_lcd_reset_ctrl(struct syna_tcm_hcd *tcm_hcd, bool on)
{
	int retval;
	static bool enabled;

	if (enabled == on) {
		input_err(true, tcm_hcd->pdev->dev.parent, "%s: skip: (%d/%d)\n", __func__, enabled, on);
		return 0;
	}

	if (on) {
		retval = regulator_enable(tcm_hcd->regulator_lcd_reset);
		if (retval) {
			input_err(true, tcm_hcd->pdev->dev.parent, "%s: Failed to enable regulator_lcd_reset: %d\n", __func__, retval);
			return retval;
		}
	} else {
		regulator_disable(tcm_hcd->regulator_lcd_reset);
	}

	enabled = on;

	input_info(true, tcm_hcd->pdev->dev.parent, "%s %d done\n", __func__, on);

	return 0;
}

static int pinctrl_configure(struct syna_tcm_hcd *tcm_hcd, bool enable)
{
	struct pinctrl_state *state;

	input_info(true, tcm_hcd->pdev->dev.parent, "%s: %s\n", __func__,
									enable ? "ACTIVE" : "SUSPEND");

	if (enable) {
		state = pinctrl_lookup_state(tcm_hcd->pinctrl, "on_state");
		if (IS_ERR(tcm_hcd->pinctrl))
			input_err(true, tcm_hcd->pdev->dev.parent,
				"%s: could not get active pinstate\n", __func__);
	} else {
		state = pinctrl_lookup_state(tcm_hcd->pinctrl, "off_state");
		if (IS_ERR(tcm_hcd->pinctrl))
			input_err(true, tcm_hcd->pdev->dev.parent,
				"%s: could not get suspend pinstate\n", __func__);
	}

	if (!IS_ERR_OR_NULL(state))
		return pinctrl_select_state(tcm_hcd->pinctrl, state);

	return 0;
}

#if defined(CONFIG_PM) || defined(CONFIG_FB)
static int syna_tcm_early_resume(struct device *dev)
{
	int retval = 0;
	struct syna_tcm_hcd *tcm_hcd = dev_get_drvdata(dev);

	input_info(true, tcm_hcd->pdev->dev.parent, "%s start(%d) (%d) (%d)\n",
				__func__, tcm_hcd->lp_state, tcm_hcd->early_resume_cnt, tcm_hcd->boot_resume);

	if (tcm_hcd->lp_state == PWR_ON && !tcm_hcd->boot_resume) {
		input_info(true, tcm_hcd->pdev->dev.parent, "%s: abnormal call!\n", __func__);
		return 0;
	}

	tcm_hcd->early_resume_cnt++;

	mutex_lock(&tcm_hcd->mode_change_mutex);
	if (tcm_hcd->lp_state == LP_MODE){
		if (tcm_hcd->irq_wake) {
			disable_irq_wake(tcm_hcd->irq);
			tcm_hcd->irq_wake = false;
		}
		retval = tcm_hcd->enable_irq(tcm_hcd, false, false);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to disable interrupt before \n");
		}

		syna_tcm_lcd_reset_ctrl(tcm_hcd, false);
		msleep(10);
	}
	mutex_unlock(&tcm_hcd->mode_change_mutex);

	input_info(true, tcm_hcd->pdev->dev.parent, "%s end\n", __func__);

	return retval;
}
static int syna_tcm_resume(struct device *dev)
{
	int retval;
	struct syna_tcm_hcd *tcm_hcd = dev_get_drvdata(dev);

	input_info(true, tcm_hcd->pdev->dev.parent, "%s start(%d) (%d)\n", __func__, tcm_hcd->lp_state, tcm_hcd->boot_resume);

	if (tcm_hcd->lp_state == PWR_ON && !tcm_hcd->boot_resume) {
		input_info(true, tcm_hcd->pdev->dev.parent, "%s: abnormal call!\n", __func__);
		return 0;
	}
	if (tcm_hcd->boot_resume)
		tcm_hcd->boot_resume = false;

	mutex_lock(&tcm_hcd->mode_change_mutex);

	pinctrl_configure(tcm_hcd, true);

	if (tcm_hcd->lp_state == LP_MODE) {
		msleep(20);
		input_info(true, tcm_hcd->pdev->dev.parent, "%s: Add 20 ms LP->ON\n", __func__);
	}

	tcm_hcd->lp_state = PWR_ON;

	tcm_hcd->prox_power_off = 0;
	tcm_hcd->enable_irq(tcm_hcd, true, NULL);

	if (tcm_hcd->in_hdl_mode) {
		retval = syna_tcm_wait_hdl(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to wait for completion of host download\n");
			goto exit;
		}
		goto mod_resume;
	} else {

#ifdef RESET_ON_RESUME
		msleep(RESET_ON_RESUME_DELAY_MS);
		goto do_reset;
#endif
	}

	if (IS_NOT_FW_MODE(tcm_hcd->id_info.mode) || tcm_hcd->app_status != APP_STATUS_OK) {
		input_info(true, tcm_hcd->pdev->dev.parent,
			"Identifying mode = 0x%02x\n", tcm_hcd->id_info.mode);
		goto do_reset;
	}

#ifdef USE_FLASH
	retval = tcm_hcd->sleep(tcm_hcd, false);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to exit deep sleep\n");
		goto exit;
	}

	retval = syna_tcm_rezero(tcm_hcd);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to rezero\n");
		goto exit;
	}

	goto mod_resume;
#endif

do_reset:
	input_info(true, tcm_hcd->pdev->dev.parent, "%s : do_reset\n", __func__);

	retval = tcm_hcd->reset_n_reinit(tcm_hcd, false, true);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to do reset and reinit\n");
		goto exit;
	}

	if (IS_NOT_FW_MODE(tcm_hcd->id_info.mode) || tcm_hcd->app_status != APP_STATUS_OK) {
		input_info(true, tcm_hcd->pdev->dev.parent, "Identifying mode = 0x%02x\n",
				tcm_hcd->id_info.mode);
		retval = 0;
		goto exit;
	}

mod_resume:
	input_info(true, tcm_hcd->pdev->dev.parent, "%s : mod_resume\n", __func__);

	if (tcm_hcd->ear_detect_enable) {
		input_info(true, tcm_hcd->pdev->dev.parent, "%s : set ed(%d)\n",
					__func__, tcm_hcd->ear_detect_enable);
		retval = tcm_hcd->set_dynamic_config(tcm_hcd, DC_ENABLE_FACE_DETECT, tcm_hcd->ear_detect_enable);
		if (retval < 0)
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to enable ear_detect mode\n");
	}

#ifdef CONFIG_SEC_FACTORY
	retval = tcm_hcd->set_dynamic_config(tcm_hcd, DC_ENABLE_EDGE_REJECT, 1);
	if (retval < 0)
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to enable edge reject\n");
	else
		input_info(true, tcm_hcd->pdev->dev.parent, "enable edge reject\n");
#endif

	if (tcm_hcd->wakeup_gesture_enabled || tcm_hcd->ear_detect_enable)
		syna_tcm_lcd_power_ctrl(tcm_hcd, false);
	if(tcm_hcd->ear_detect_enable)
		syna_tcm_lcd_reset_ctrl(tcm_hcd, false);


#ifdef WATCHDOG_SW
	tcm_hcd->update_watchdog(tcm_hcd, true);
#endif
	cancel_delayed_work(&tcm_hcd->work_print_info);
	tcm_hcd->print_info_cnt_open = 0;
	tcm_hcd->print_info_cnt_release = 0;
	if (!shutdown_is_on_going_tsp)
		schedule_work(&tcm_hcd->work_print_info.work);

	tcm_hcd->wakeup_gesture_enabled = tcm_hcd->aot_enable;
	tcm_hcd->prox_power_off = 0;
	retval = 0;

	input_info(true, tcm_hcd->pdev->dev.parent, "%s end\n", __func__);

exit:
	tcm_hcd->early_resume_cnt = 0;
	tcm_hcd->prox_lp_scan_cnt = 0;
	mutex_unlock(&tcm_hcd->mode_change_mutex);

	return retval;
}


static int syna_tcm_early_suspend(struct device *dev)
{
	int retval;
	struct syna_tcm_hcd *tcm_hcd = dev_get_drvdata(dev);

	input_info(true, tcm_hcd->pdev->dev.parent, "%s start(%d)\n", __func__, tcm_hcd->lp_state);

	if (tcm_hcd->lp_state != PWR_ON) {
		input_info(true, tcm_hcd->pdev->dev.parent, "%s: abnormal call!\n", __func__);
		return 0;
	}

	mutex_lock(&tcm_hcd->mode_change_mutex);

#ifdef WATCHDOG_SW
	tcm_hcd->update_watchdog(tcm_hcd, false);
#endif
	if (tcm_hcd->aot_enable && tcm_hcd->prox_power_off)
		tcm_hcd->wakeup_gesture_enabled = 0;

	if (tcm_hcd->wakeup_gesture_enabled || tcm_hcd->lcdoff_test) {
		input_info(true, tcm_hcd->pdev->dev.parent, "Enter lp mode(aot)\n");
		syna_tcm_lcd_power_ctrl(tcm_hcd, true);
		syna_tcm_lcd_reset_ctrl(tcm_hcd, true);

	} else if (tcm_hcd->ear_detect_enable) {
		input_info(true, tcm_hcd->pdev->dev.parent, "Enter lp mode(ed)\n");
		syna_tcm_lcd_power_ctrl(tcm_hcd, true);
		syna_tcm_lcd_reset_ctrl(tcm_hcd, true);

	} else {
		retval = tcm_hcd->enable_irq(tcm_hcd, false, false);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to disable interrupt before \n");
		}
		pinctrl_configure(tcm_hcd, false);
	}

	if (IS_NOT_FW_MODE(tcm_hcd->id_info.mode) || tcm_hcd->app_status != APP_STATUS_OK) {
		input_info(true, tcm_hcd->pdev->dev.parent,
				"Identifying mode = 0x%02x\n", tcm_hcd->id_info.mode);
		mutex_unlock(&tcm_hcd->mode_change_mutex);
		return 0;
	}

#ifdef USE_FLASH
	if (!tcm_hcd->wakeup_gesture_enabled || tcm_hcd->lcdoff_test) {
		retval = tcm_hcd->sleep(tcm_hcd, true);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to enter deep sleep\n");
			mutex_unlock(&tcm_hcd->mode_change_mutex);
			return retval;
		}
	}
#endif

	cancel_delayed_work(&tcm_hcd->work_print_info);
	sec_ts_print_info(tcm_hcd);

	touch_free_objects();
	mutex_unlock(&tcm_hcd->mode_change_mutex);

	input_info(true, tcm_hcd->pdev->dev.parent, "%s done\n", __func__);

	return 0;
}

static int syna_tcm_suspend(struct device *dev)
{
	struct syna_tcm_hcd *tcm_hcd = dev_get_drvdata(dev);
	int retval;

	input_info(true, tcm_hcd->pdev->dev.parent, "%s start(%d) aot(%d/%d) ed(%d)\n",
				__func__, tcm_hcd->lp_state, tcm_hcd->aot_enable, tcm_hcd->wakeup_gesture_enabled,
				tcm_hcd->ear_detect_enable);

	if (tcm_hcd->lp_state != PWR_ON) {
		input_info(true, tcm_hcd->pdev->dev.parent, "%s: abnormal call!\n", __func__);
		return 0;
	}

	mutex_lock(&tcm_hcd->mode_change_mutex);

	if (tcm_hcd->wakeup_gesture_enabled || tcm_hcd->lcdoff_test) {
		input_info(true, tcm_hcd->pdev->dev.parent, "Enter lp mode(aot)\n");
		tcm_hcd->lp_state = LP_MODE;

		if (!tcm_hcd->irq_wake) {
			enable_irq_wake(tcm_hcd->irq);
			tcm_hcd->irq_wake = true;
		}

		if (tcm_hcd->ear_detect_enable) {
			input_info(true, tcm_hcd->pdev->dev.parent, "%s: ed off before aot set\n", __func__);
			retval = tcm_hcd->set_dynamic_config(tcm_hcd, DC_ENABLE_FACE_DETECT, 0);
			if (retval < 0) {
				input_err(true, tcm_hcd->pdev->dev.parent,
						"%s: Failed to enable ear detect mode\n", __func__);
			}
		}

		retval = tcm_hcd->set_dynamic_config(tcm_hcd, DC_IN_WAKEUP_GESTURE_MODE, 1);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to enable wakeup gesture mode\n");
			touch_free_objects();
			mutex_unlock(&tcm_hcd->mode_change_mutex);
			return retval;
		}

	} else if (tcm_hcd->ear_detect_enable) {
		input_info(true, tcm_hcd->pdev->dev.parent, "Enter lp mode(ed)\n");
		tcm_hcd->lp_state = LP_MODE;

		if (!tcm_hcd->irq_wake) {
			enable_irq_wake(tcm_hcd->irq);
			tcm_hcd->irq_wake = true;
		}

	} else {
		retval = tcm_hcd->enable_irq(tcm_hcd, false, false);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to disable interrupt before \n");
		}
		input_info(true, tcm_hcd->pdev->dev.parent, "Enter power off\n");
		tcm_hcd->lp_state = PWR_OFF;
		pinctrl_configure(tcm_hcd, false);
	}

	touch_free_objects();

	if(tcm_hcd->prox_lp_scan_cnt > 0) {
		input_info(true, tcm_hcd->pdev->dev.parent, "%s: scan start!\n", __func__);
		retval = syna_tcm_set_scan_start_stop_cmd(tcm_hcd, 1);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent,
				"Failed to write command %s\n", STR(CMD_SET_SCAN_START_STOP));
		}
	}

	mutex_unlock(&tcm_hcd->mode_change_mutex);

	input_info(true, tcm_hcd->pdev->dev.parent, "%s done\n", __func__);

	return 0;
}

static int syna_tcm_pm_suspend(struct device *dev)
{
	struct syna_tcm_hcd *tcm_hcd = dev_get_drvdata(dev);

	reinit_completion(&tcm_hcd->resume_done);

	return 0;
}

static int syna_tcm_pm_resume(struct device *dev)
{
	struct syna_tcm_hcd *tcm_hcd = dev_get_drvdata(dev);

	complete_all(&tcm_hcd->resume_done);

	return 0;
}


static int syna_tcm_fb_notifier_cb(struct notifier_block *nb,
		unsigned long action, void *data)
{
	int retval;
	int *transition;
	struct fb_event *evdata = data;
	struct syna_tcm_hcd *tcm_hcd = container_of(nb, struct syna_tcm_hcd, fb_notifier);

	retval = 0;
	if (evdata && evdata->data && tcm_hcd) {
		transition = evdata->data;

		if (atomic_read(&tcm_hcd->firmware_flashing) && *transition == FB_BLANK_POWERDOWN) {

			retval = wait_event_interruptible_timeout(tcm_hcd->reflash_wq,
						!atomic_read(&tcm_hcd->firmware_flashing),
						msecs_to_jiffies(RESPONSE_TIMEOUT_MS));
			if (retval == 0) {
				input_err(true, tcm_hcd->pdev->dev.parent,
					"Timed out waiting for completion of flashing firmware\n");
				atomic_set(&tcm_hcd->firmware_flashing, 0);
				return -EIO;
			} else {
				retval = 0;
			}
		}

		if (action == FB_EARLY_EVENT_BLANK && *transition == FB_BLANK_POWERDOWN)
			retval = syna_tcm_early_suspend(&tcm_hcd->pdev->dev);
		else if (action == FB_EVENT_BLANK) {
			if (*transition == FB_BLANK_POWERDOWN) {
				retval = syna_tcm_suspend(&tcm_hcd->pdev->dev);
				tcm_hcd->fb_ready = 0;
			} else if (*transition == FB_BLANK_UNBLANK) {
//#ifndef RESUME_EARLY_UNBLANK
				zeroflash_on_panel_enabled(tcm_hcd);
				retval = syna_tcm_resume(&tcm_hcd->pdev->dev);
				tcm_hcd->fb_ready++;
//#endif
			}
		} else if (action == FB_EARLY_EVENT_BLANK &&
				*transition == FB_BLANK_UNBLANK) {
//#ifdef RESUME_EARLY_UNBLANK
				zeroflash_on_panel_enabled(tcm_hcd);
				retval = syna_tcm_early_resume(&tcm_hcd->pdev->dev);
				tcm_hcd->fb_ready++;
//#endif
		}
	}

	return 0;
}
#endif

static int syna_tcm_check_f35(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	unsigned char fn_number;
	int retry = 0;
	const int retry_max = 10;

f35_boot_recheck:
			retval = syna_tcm_rmi_read(tcm_hcd, PDT_END_ADDR, &fn_number, sizeof(fn_number));
			if (retval < 0) {
				input_err(true, tcm_hcd->pdev->dev.parent,
					"Failed to read F35 function number\n");
				tcm_hcd->is_detected = false;
				return -ENODEV;
			}

			input_dbg(true, tcm_hcd->pdev->dev.parent, "Found F$%02x\n", fn_number);

			if (fn_number != RMI_UBL_FN_NUMBER) {
					input_err(true, tcm_hcd->pdev->dev.parent,
						"Failed to find F$35, try_times = %d\n", retry);
				if (retry < retry_max) {
					msleep(100);
					retry++;
			goto f35_boot_recheck;
				}
				tcm_hcd->is_detected = false;
				return -ENODEV;
			}
	return 0;
}

static int syna_tcm_sensor_detection(struct syna_tcm_hcd *tcm_hcd)
{
	int retval;
	unsigned char *build_id;
	unsigned int payload_length;
	unsigned int max_write_size;

	tcm_hcd->in_hdl_mode = false;
	tcm_hcd->sensor_type = TYPE_UNKNOWN;

	/* read sensor info for identification */
	retval = tcm_hcd->read_message(tcm_hcd, NULL, 0);

	/* once the tcm communication interface is not ready, */
	/* check whether the device is in F35 mode        */
	if (retval < 0) {
		if (retval == -ENXIO && tcm_hcd->hw_if->bus_io->type == BUS_SPI) {

			retval = syna_tcm_check_f35(tcm_hcd);
			if (retval < 0) {
				input_err(true, tcm_hcd->pdev->dev.parent, "Failed to read TCM message\n");
				return retval;
			}
			tcm_hcd->in_hdl_mode = true;
			tcm_hcd->sensor_type = TYPE_F35;
			tcm_hcd->is_detected = true;
			tcm_hcd->rd_chunk_size = HDL_RD_CHUNK_SIZE;
			tcm_hcd->wr_chunk_size = HDL_WR_CHUNK_SIZE;
			input_info(true, tcm_hcd->pdev->dev.parent, "F35 mode\n");

			return retval;
		} else {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to read TCM message\n");

			return retval;
		}
	}

	/* expect to get an identify report after powering on */

	if (tcm_hcd->status_report_code != REPORT_IDENTIFY) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Unexpected report code (0x%02x)\n", tcm_hcd->status_report_code);

		return -ENODEV;
	}

	tcm_hcd->is_detected = true;
	payload_length = tcm_hcd->payload_length;

	LOCK_BUFFER(tcm_hcd->in);

	retval = secure_memcpy((unsigned char *)&tcm_hcd->id_info,
				sizeof(tcm_hcd->id_info), &tcm_hcd->in.buf[MESSAGE_HEADER_SIZE],
				tcm_hcd->in.buf_size - MESSAGE_HEADER_SIZE,
				MIN(sizeof(tcm_hcd->id_info), payload_length));
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to copy identification info\n");
		UNLOCK_BUFFER(tcm_hcd->in);
		return retval;
	}

	syna_tcm_dump_identify(tcm_hcd, &tcm_hcd->in.buf[MESSAGE_HEADER_SIZE],
		payload_length, "cold-boot leftover/sensor_detection (no 0x40)");

	UNLOCK_BUFFER(tcm_hcd->in);

	build_id = tcm_hcd->id_info.build_id;
	tcm_hcd->packrat_number = le4_to_uint(build_id);

	max_write_size = le2_to_uint(tcm_hcd->id_info.max_write_size);
	tcm_hcd->wr_chunk_size = MIN(max_write_size, WR_CHUNK_SIZE);
	if (tcm_hcd->wr_chunk_size == 0)
		tcm_hcd->wr_chunk_size = max_write_size;

	if (tcm_hcd->id_info.mode == MODE_ROMBOOTLOADER) {
		tcm_hcd->in_hdl_mode = true;
		tcm_hcd->sensor_type = TYPE_ROMBOOT;
		tcm_hcd->rd_chunk_size = HDL_RD_CHUNK_SIZE;
		/* Normal TCM wr_chunk stays MIN(max_write, WR_CHUNK_SIZE)=512.
		 * Do not set wr_chunk=1024 globally for HDL. One-shot is only
		 * around write_message(CMD_ROMBOOT_DOWNLOAD).
		 */
		pr_info("SAaiOS_TOUCH_DBG: leftover IDENTIFY wr_chunk_size=%u max_write=%u WR_CHUNK_SIZE=%u HDL_WR_CHUNK_SIZE=%u (normal TCM MIN(max_write,WR_CHUNK)=512; oneshot only around 0x45)\n",
			tcm_hcd->wr_chunk_size, max_write_size, WR_CHUNK_SIZE,
			HDL_WR_CHUNK_SIZE);
		input_info(true, tcm_hcd->pdev->dev.parent, "RomBoot mode\n");
	} else if (tcm_hcd->id_info.mode == MODE_APPLICATION_FIRMWARE) {
		tcm_hcd->sensor_type = TYPE_FLASH;
		input_info(true, tcm_hcd->pdev->dev.parent,
			"Application mode (build id = %d)\n", tcm_hcd->packrat_number);
	} else {
		input_info(true, tcm_hcd->pdev->dev.parent,
			"TCM is detected, but mode is 0x%02x\n", tcm_hcd->id_info.mode);
	}

	return 0;
}

static int syna_tcm_probe(struct platform_device *pdev)
{
	int retval;
	struct syna_tcm_hcd *tcm_hcd;
	const struct syna_tcm_board_data *bdata;
	const struct syna_tcm_hw_interface *hw_if;

	pr_info("SAaiOS_TOUCH_DBG: since76 touch lab: read-floor 256 + touchlab, skip 0x25 after 0x20 OK, auto live20 ladder 10/100/500/1000ms after REINIT unlock (empty GET only; no delay=0 race; no auto 0x05/0x24/0x30), retval<0→dead response=ff, tokens live20/run_app/enable_report/no_doze (app_config optional not menu), oneshot 0x45, IDENTIFY 0x02 STATUS_OK, skip 0x1f HDL firmware running, no 0x40\n");
	pr_info("SAaiOS_TOUCH_DBG: probe enter\n");

	hw_if = pdev->dev.platform_data;
	if (!hw_if) {
		input_err(true, &pdev->dev, "Hardware interface not found\n");
		return -ENODEV;
	}

	bdata = hw_if->bdata;
	if (!bdata) {
		input_err(true, &pdev->dev, "Board data not found\n");
		return -ENODEV;
	}

	tcm_hcd = kzalloc(sizeof(*tcm_hcd), GFP_KERNEL);
	if (!tcm_hcd) {
		input_err(true, &pdev->dev, "Failed to allocate memory for tcm_hcd\n");
		return -ENOMEM;
	}

	saaios_storm_tcm = tcm_hcd;
	platform_set_drvdata(pdev, tcm_hcd);

	tcm_hcd->pinctrl = bdata->pinctrl;
	tcm_hcd->pdev = pdev;
	tcm_hcd->hw_if = hw_if;
	tcm_hcd->reset = syna_tcm_reset;
	tcm_hcd->reset_n_reinit = syna_tcm_reset_and_reinit;
	tcm_hcd->sleep = syna_tcm_sleep;
	tcm_hcd->identify = syna_tcm_identify;
	tcm_hcd->enable_irq = syna_tcm_enable_irq;
	tcm_hcd->switch_mode = syna_tcm_switch_mode;
	tcm_hcd->read_message = syna_tcm_read_message;
	tcm_hcd->write_message = syna_tcm_write_message;
	tcm_hcd->get_dynamic_config = syna_tcm_get_dynamic_config;
	tcm_hcd->set_dynamic_config = syna_tcm_set_dynamic_config;
	tcm_hcd->get_data_location = syna_tcm_get_data_location;

	tcm_hcd->rd_chunk_size = RD_CHUNK_SIZE;
	tcm_hcd->wr_chunk_size = WR_CHUNK_SIZE;
	tcm_hcd->is_detected = false;
	tcm_hcd->lp_state = PWR_ON;
	tcm_hcd->boot_resume = true;
/*	tcm_hcd->wakeup_gesture_enabled = WAKEUP_GESTURE; */

#ifdef PREDICTIVE_READING
	/* Probe-only initial floor (not the removed global per-message clamp). */
	tcm_hcd->read_length = 256;
#else
	tcm_hcd->read_length = MESSAGE_HEADER_SIZE;
#endif

#ifdef WATCHDOG_SW
	tcm_hcd->watchdog.run = RUN_WATCHDOG;
	tcm_hcd->update_watchdog = syna_tcm_update_watchdog;
#endif

	if (bdata->irq_gpio >= 0)
		tcm_hcd->irq = gpio_to_irq(bdata->irq_gpio);
	else
		tcm_hcd->irq = bdata->irq_gpio;

	mutex_init(&tcm_hcd->extif_mutex);
	mutex_init(&tcm_hcd->reset_mutex);
	mutex_init(&tcm_hcd->irq_en_mutex);
	mutex_init(&tcm_hcd->io_ctrl_mutex);
	mutex_init(&tcm_hcd->rw_ctrl_mutex);
	mutex_init(&tcm_hcd->command_mutex);
	mutex_init(&tcm_hcd->identify_mutex);
	mutex_init(&tcm_hcd->mode_change_mutex);

	INIT_BUFFER(tcm_hcd->in, false);
	INIT_BUFFER(tcm_hcd->out, false);
	INIT_BUFFER(tcm_hcd->resp, true);
	INIT_BUFFER(tcm_hcd->temp, false);
	INIT_BUFFER(tcm_hcd->config, false);
	INIT_BUFFER(tcm_hcd->report.buffer, true);

	LOCK_BUFFER(tcm_hcd->in);

	retval = syna_tcm_alloc_mem(tcm_hcd, &tcm_hcd->in, tcm_hcd->read_length + 1);
	if (retval < 0) {
		input_err(true, &pdev->dev, "Failed to allocate memory for tcm_hcd->in.buf\n");
		UNLOCK_BUFFER(tcm_hcd->in);
		goto err_alloc_mem;
	}

	UNLOCK_BUFFER(tcm_hcd->in);

	atomic_set(&tcm_hcd->command_status, CMD_IDLE);

	atomic_set(&tcm_hcd->helper.task, HELP_NONE);

	device_init_wakeup(&pdev->dev, 1);

	init_waitqueue_head(&tcm_hcd->hdl_wq);

	init_waitqueue_head(&tcm_hcd->reflash_wq);
	atomic_set(&tcm_hcd->firmware_flashing, 0);

	if (!mod_pool.initialized) {
		mutex_init(&mod_pool.mutex);
		INIT_LIST_HEAD(&mod_pool.list);
		mod_pool.initialized = true;
	}

	retval = syna_tcm_get_regulator(tcm_hcd, true);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to get regulators\n");
		goto err_get_regulator;
	}

	retval = syna_tcm_enable_regulator(tcm_hcd, true);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to enable regulators\n");
		goto err_enable_regulator;
	}

	retval = syna_tcm_config_gpio(tcm_hcd);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to configure GPIO's\n");
		goto err_config_gpio;
	}

	retval = syna_tcm_get_lcd_regulator(tcm_hcd, true);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to get regulators\n");
		goto err_get_lcd_regulator;
	}

	pinctrl_configure(tcm_hcd, true);

	/* detect the type of touch controller */
	retval = syna_tcm_sensor_detection(tcm_hcd);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to detect the sensor\n");
		goto err_get_lcd_regulator;
	}

#ifdef CONFIG_FB
	tcm_hcd->fb_notifier.notifier_call = syna_tcm_fb_notifier_cb;
	retval = fb_register_client(&tcm_hcd->fb_notifier);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to register FB notifier client\n");
	}
#endif

#ifdef REPORT_NOTIFIER
	tcm_hcd->notifier_thread = kthread_run(syna_tcm_report_notifier, tcm_hcd,
										"syna_tcm_report_notifier");
	if (IS_ERR(tcm_hcd->notifier_thread)) {
		retval = PTR_ERR(tcm_hcd->notifier_thread);
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to create and run tcm_hcd->notifier_thread\n");
		goto err_create_run_kthread;
	}
#endif

	tcm_hcd->helper.workqueue = create_singlethread_workqueue("syna_tcm_helper");
	INIT_WORK(&tcm_hcd->helper.work, syna_tcm_helper_work);

#ifdef WATCHDOG_SW
	tcm_hcd->watchdog.workqueue = create_singlethread_workqueue("syna_tcm_watchdog");
	INIT_DELAYED_WORK(&tcm_hcd->watchdog.work, syna_tcm_watchdog_work);
#endif

	tcm_hcd->polling_workqueue = create_singlethread_workqueue("syna_tcm_polling");
	INIT_DELAYED_WORK(&tcm_hcd->polling_work, syna_tcm_polling_work);

	INIT_DELAYED_WORK(&tcm_hcd->work_print_info, touch_print_info_work);

	retval = syna_tcm_saaios_exp_init(tcm_hcd);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent,
			"Failed to init saaios_touch_wq\n");
		goto err_saaios_exp_init;
	}

	/* skip the following initialization */
	/* since the fw is not ready for hdl devices */
	if (tcm_hcd->in_hdl_mode) {
		retval = zeroflash_init(tcm_hcd);
		if (retval < 0) {
			input_err(true, tcm_hcd->pdev->dev.parent, "Failed to zeroflash init\n");
			goto err_zeroflash_init;
		}
		/* goto prepare_modules; */
	}

	/* register and enable the interrupt in probe */
	/* if this is not the hdl device */
	retval = tcm_hcd->enable_irq(tcm_hcd, true, NULL);
	if (retval < 0) {
		input_err(true, tcm_hcd->pdev->dev.parent, "Failed to enable interrupt\n");
		goto err_enable_irq;
	}
	input_dbg(true, tcm_hcd->pdev->dev.parent, "Interrupt is registered\n");

	/* since59: do not call identify(false) — that is 0x40 from RomBoot
	 * 0x04 and contaminates the IC to on-chip HDL 0x02. Do not
	 * touch_init / 0x25. Stay bound so delayed 0x45 can run after panel.
	 */
	pr_info("SAaiOS_TOUCH_DBG: skip identify(false)/0x40 and touch_init/0x25 (stay bound, wait panel)\n");
	syna_tcm_since58_observe(tcm_hcd, "after probe (no 0x40)");
	if (!shutdown_is_on_going_tsp)
		schedule_delayed_work(&tcm_hcd->work_print_info, msecs_to_jiffies(5000));

	/* prepare to add other modules */
	mod_pool.workqueue = create_singlethread_workqueue("syna_tcm_module");
	INIT_WORK(&mod_pool.work, syna_tcm_module_work);
	mod_pool.tcm_hcd = tcm_hcd;
	mod_pool.queue_work = true;
	queue_work(mod_pool.workqueue, &mod_pool.work);

	INIT_DELAYED_WORK(&tcm_hcd->work_read_info, sec_read_info_work);
	/* no factory 0x2a before 0x45 observation */

	init_completion(&tcm_hcd->resume_done);
	complete_all(&tcm_hcd->resume_done);

	return 0;

err_enable_irq:
	zeroflash_remove(tcm_hcd);
err_zeroflash_init:
	syna_tcm_saaios_exp_exit();
err_saaios_exp_init:
	cancel_delayed_work_sync(&tcm_hcd->polling_work);
	flush_workqueue(tcm_hcd->polling_workqueue);
	destroy_workqueue(tcm_hcd->polling_workqueue);

#ifdef WATCHDOG_SW
	cancel_delayed_work_sync(&tcm_hcd->watchdog.work);
	flush_workqueue(tcm_hcd->watchdog.workqueue);
	destroy_workqueue(tcm_hcd->watchdog.workqueue);
#endif

	cancel_work_sync(&tcm_hcd->helper.work);
	flush_workqueue(tcm_hcd->helper.workqueue);
	destroy_workqueue(tcm_hcd->helper.workqueue);

#ifdef REPORT_NOTIFIER
	kthread_stop(tcm_hcd->notifier_thread);

err_create_run_kthread:
#endif
#ifdef CONFIG_FB
	fb_unregister_client(&tcm_hcd->fb_notifier);
#endif

	if (bdata->irq_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->irq_gpio, false, 0, 0);

	if (bdata->cs_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->cs_gpio, false, 0, 0);

	if (bdata->power_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->power_gpio, false, 0, 0);

	if (bdata->reset_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->reset_gpio, false, 0, 0);
err_get_lcd_regulator:
	syna_tcm_get_lcd_regulator(tcm_hcd, false);

err_config_gpio:
	syna_tcm_enable_regulator(tcm_hcd, false);

err_enable_regulator:
	syna_tcm_get_regulator(tcm_hcd, false);

err_get_regulator:
	device_init_wakeup(&pdev->dev, 0);

err_alloc_mem:
	RELEASE_BUFFER(tcm_hcd->report.buffer);
	RELEASE_BUFFER(tcm_hcd->config);
	RELEASE_BUFFER(tcm_hcd->temp);
	RELEASE_BUFFER(tcm_hcd->resp);
	RELEASE_BUFFER(tcm_hcd->out);
	RELEASE_BUFFER(tcm_hcd->in);

	kfree(tcm_hcd);

	return retval;
}

static int syna_tcm_remove(struct platform_device *pdev)
{
	struct syna_tcm_module_handler *mod_handler;
	struct syna_tcm_module_handler *tmp_handler;
	struct syna_tcm_hcd *tcm_hcd = platform_get_drvdata(pdev);
	const struct syna_tcm_board_data *bdata = tcm_hcd->hw_if->bdata;

	input_info(true, pdev->dev.parent, "%s\n", __func__);
	shutdown_is_on_going_tsp = true;

	if (tcm_hcd->irq_enabled && bdata->irq_gpio >= 0) {
		disable_irq(tcm_hcd->irq);
		free_irq(tcm_hcd->irq, tcm_hcd);
	}

	touch_remove(tcm_hcd);

	mutex_lock(&mod_pool.mutex);

	if (!list_empty(&mod_pool.list)) {
		list_for_each_entry_safe(mod_handler, tmp_handler, &mod_pool.list, link) {
			if (mod_handler->mod_cb->remove)
				mod_handler->mod_cb->remove(tcm_hcd);
			list_del(&mod_handler->link);
			kfree(mod_handler);
		}
	}

	mod_pool.queue_work = false;
	cancel_work_sync(&mod_pool.work);
	flush_workqueue(mod_pool.workqueue);
	destroy_workqueue(mod_pool.workqueue);

	mutex_unlock(&mod_pool.mutex);

	cancel_delayed_work_sync(&tcm_hcd->polling_work);
	flush_workqueue(tcm_hcd->polling_workqueue);
	destroy_workqueue(tcm_hcd->polling_workqueue);

	cancel_delayed_work_sync(&tcm_hcd->work_print_info);
	cancel_delayed_work_sync(&tcm_hcd->work_read_info);

#ifdef WATCHDOG_SW
	cancel_delayed_work_sync(&tcm_hcd->watchdog.work);
	flush_workqueue(tcm_hcd->watchdog.workqueue);
	destroy_workqueue(tcm_hcd->watchdog.workqueue);
#endif

	cancel_work_sync(&tcm_hcd->helper.work);
	flush_workqueue(tcm_hcd->helper.workqueue);
	destroy_workqueue(tcm_hcd->helper.workqueue);

	syna_tcm_saaios_exp_exit();

#ifdef REPORT_NOTIFIER
	kthread_stop(tcm_hcd->notifier_thread);
#endif

#ifdef CONFIG_FB
	fb_unregister_client(&tcm_hcd->fb_notifier);
#endif

	if (bdata->irq_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->irq_gpio, false, 0, 0);

	if (bdata->cs_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->cs_gpio, false, 0, 0);

	if (bdata->power_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->power_gpio, false, 0, 0);

	if (bdata->reset_gpio >= 0)
		syna_tcm_set_gpio(tcm_hcd, bdata->reset_gpio, false, 0, 0);

	syna_tcm_enable_regulator(tcm_hcd, false);

	syna_tcm_get_regulator(tcm_hcd, false);
	syna_tcm_get_lcd_regulator(tcm_hcd, false);

	device_init_wakeup(&pdev->dev, 0);

	RELEASE_BUFFER(tcm_hcd->report.buffer);
	RELEASE_BUFFER(tcm_hcd->config);
	RELEASE_BUFFER(tcm_hcd->temp);
	RELEASE_BUFFER(tcm_hcd->resp);
	RELEASE_BUFFER(tcm_hcd->out);
	RELEASE_BUFFER(tcm_hcd->in);

	sec_fn_remove(tcm_hcd);

	kfree(tcm_hcd);

	return 0;
}

static void syna_tcm_shutdown(struct platform_device *pdev)
{
	int retval;

	retval = syna_tcm_remove(pdev);
}

#ifdef CONFIG_PM
static const struct dev_pm_ops syna_tcm_dev_pm_ops = {
/*#ifndef CONFIG_FB*/
	.suspend = syna_tcm_pm_suspend,
	.resume = syna_tcm_pm_resume,
/*#endif*/
};
#endif

static struct platform_driver syna_tcm_driver = {
	.driver = {
		.name = PLATFORM_DRIVER_NAME,
		.owner = THIS_MODULE,
#ifdef CONFIG_PM
		.pm = &syna_tcm_dev_pm_ops,
#endif
	},
	.probe = syna_tcm_probe,
	.remove = syna_tcm_remove,
	.shutdown = syna_tcm_shutdown,
};

static int __init syna_tcm_module_init(void)
{
	int retval;

	retval = syna_tcm_bus_init();
	if (retval < 0)
		return retval;

	return platform_driver_register(&syna_tcm_driver);
}

static void __exit syna_tcm_module_exit(void)
{
	platform_driver_unregister(&syna_tcm_driver);

	syna_tcm_bus_exit();

	return;
}

module_init(syna_tcm_module_init);
module_exit(syna_tcm_module_exit);

MODULE_AUTHOR("Synaptics, Inc.");
MODULE_DESCRIPTION("Synaptics TCM Touch Driver");
MODULE_LICENSE("GPL v2");
