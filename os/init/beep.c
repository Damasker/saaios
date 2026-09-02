/* SaaiOS /sbin/beep — SMA1303 speaker via ABOX RDMA1 / SIFS1.
 * v025 LIVE: TONEGEN on RDMA3_A + SIFS0→UAIF1 attached (Calliope NFB0,
 * UNMUTE, rdma_trigger[3]) and still silent — not a None mux. pcm_writei
 * EIO on D3 with TONEGEN occupying the slot.
 * v026: pcmC0D1p + vendor route-rdma*-to-sifs1 / S10 GSI UAIF1=SIFS1.
 * Do not toggle Codec Enable, SMA I2C reset, HP/EP/codec SPK,
 * Force AMP Power Down.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <linux/input.h>
#include <tinyalsa/asoundlib.h>

#define CARD 0
#define DEVICE 1 /* pcmC0D1p RDMA1 — S10 GSI speaker; v025 D3 still silent */
#define RATE 48000
#define CHANNELS 2
#define PERIOD_SIZE 960
#define PERIOD_COUNT 4
#define BEEP_MS 1200
#define TONE_HZ 880
#define TONE_AMP 18000
/* Speaker Volume SOC_SINGLE_TLV invert: userspace 167 = hardware 0 (loudest).
 * Stock init-vol 0x31 shows as 118 here. 32 is quieter. 160 is loud. */
#define SPK_VOL 160

static void set_enum(struct mixer *m, const char *name, const char *val)
{
	struct mixer_ctl *c = mixer_get_ctl_by_name(m, name);
	int r;
	if (!c) {
		fprintf(stderr, "beep: missing %s\n", name);
		return;
	}
	r = mixer_ctl_set_enum_by_string(c, val);
	if (r)
		fprintf(stderr, "beep: %s=%s failed %d\n", name, val, r);
}

static void set_int(struct mixer *m, const char *name, int v)
{
	struct mixer_ctl *c = mixer_get_ctl_by_name(m, name);
	int r;
	if (!c) {
		fprintf(stderr, "beep: missing %s\n", name);
		return;
	}
	r = mixer_ctl_set_value(c, 0, v);
	if (r)
		fprintf(stderr, "beep: %s=%d failed %d\n", name, v, r);
}

static void log_jack(void)
{
	/* AUD3004X 5-pin ADC jack (event7). Reports SW_* only — does not
	 * mute SMA1303. False insert would make Android pick HEADSET.
	 */
	DIR *d = opendir("/sys/class/input");
	struct dirent *e;
	if (!d)
		return;
	while ((e = readdir(d))) {
		char path[128], name[64];
		int fd, n;
		unsigned long sw[2];
		if (strncmp(e->d_name, "event", 5))
			continue;
		snprintf(path, sizeof(path), "/sys/class/input/%s/device/name", e->d_name);
		fd = open(path, O_RDONLY);
		if (fd < 0)
			continue;
		n = read(fd, name, sizeof(name) - 1);
		close(fd);
		if (n <= 0)
			continue;
		name[n] = 0;
		if (!strstr(name, "Headset") && !strstr(name, "AUD3004"))
			continue;
		snprintf(path, sizeof(path), "/dev/input/%s", e->d_name);
		fd = open(path, O_RDONLY | O_NONBLOCK);
		if (fd < 0)
			continue;
		memset(sw, 0, sizeof(sw));
		if (ioctl(fd, EVIOCGSW(sizeof(sw)), sw) == 0)
			fprintf(stderr, "beep: jack %s hp=%d mic=%d\n", e->d_name,
				!!(sw[0] & (1ul << SW_HEADPHONE_INSERT)),
				!!(sw[0] & (1ul << SW_MICROPHONE_INSERT)));
		close(fd);
		break;
	}
	closedir(d);
}

static void route_speaker(struct mixer *m)
{
	/* Exact names from vendor mixer_paths.xml + sma1303.c.
	 * Leave ABOX SPUS ASRC3 On (stock). Turning it Off wedged pcm_writei.
	 * Idle Mode Off / Mute On is sma1303_shutdown — expected after close.
	 * Live v021: ABOX Sound Type defaulted to VOICE (receiver). Calliope
	 * IPC ABOX_SET_TYPE — that is the output-device gate, not SMA mute.
	 * Jack driver only reports SW_HEADPHONE_INSERT; it does not mute SPK.
	 */
	log_jack();
	set_enum(m, "ABOX Sound Type", "SPEAKER");
	set_enum(m, "ABOX UAIF0 SPK", "RESERVED");
	set_int(m, "HP HP On", 0);
	set_int(m, "EP EP On", 0);
	/* Vendor media-speaker is RDMA3→SIFS0→UAIF1. That path triggered
	 * on v025 and stayed silent. route-rdma3-to-sifs1 + S10 GSI used
	 * SIFS1 on UAIF1. RDMA1 (pcmC0D1p) feeds SPUS OUT1.
	 */
	set_enum(m, "ABOX SPUS OUT3", "RESERVED");
	set_enum(m, "ABOX RDMA3_A", "None");
	set_enum(m, "ABOX SPUS OUT1", "SIFS1");
	set_enum(m, "ABOX SIFS1", "SPUS OUT1");
	set_enum(m, "ABOX UAIF1 SPK", "SIFS1");
	set_int(m, "ABOX SIFS1 OUT Switch", 1);
	set_int(m, "ABOX UAIF1 Width", 16);
	set_int(m, "ABOX UAIF1 Channel", 2);
	set_int(m, "ABOX UAIF1 Rate", RATE);
	set_int(m, "ABOX UAIF1 Extend BCLK", 1);
	set_int(m, "ABOX SIFS1 Width", 16);
	set_int(m, "ABOX TONEGEN_1KHZ", 1);
	set_enum(m, "ABOX RDMA1_A", "TONEGEN_1KHZ");
	set_int(m, "Speaker Volume", SPK_VOL);
	set_int(m, "Speaker Mute Switch(1:muted_0:un)", 0);
	set_int(m, "Power Up(1:Up_0:Down)", 1);
}

static void amp_during_pcm(struct mixer *m)
{
	/* startup() already picked Mono vs Stereo. Force Mono + unmute after
	 * pcm_open so DAPM/startup cannot leave Mode Off for the write.
	 * Enum index 1 = Mono (string set is flaky with duplicate Reserved).
	 * Hold Power Up for the whole write — sma1303_shutdown is close only.
	 */
	set_int(m, "Speaker Mode", 1);
	set_int(m, "Speaker Mute Switch(1:muted_0:un)", 0);
	set_int(m, "Power Up(1:Up_0:Down)", 1);
	set_int(m, "ABOX TONEGEN_1KHZ", 1);
	set_enum(m, "ABOX RDMA1_A", "TONEGEN_1KHZ");
	set_enum(m, "ABOX UAIF1 SPK", "SIFS1");
}

int main(void)
{
	struct mixer *m;
	struct pcm *pcm;
	struct pcm_config cfg;
	unsigned frames, n, period;
	short *buf;
	int err = 1;

	m = mixer_open(CARD);
	if (!m) {
		fprintf(stderr, "beep: mixer_open card%d failed\n", CARD);
		return 1;
	}
	route_speaker(m);

	memset(&cfg, 0, sizeof(cfg));
	cfg.channels = CHANNELS;
	cfg.rate = RATE;
	cfg.period_size = PERIOD_SIZE;
	cfg.period_count = PERIOD_COUNT;
	cfg.format = PCM_FORMAT_S16_LE;
	cfg.start_threshold = PERIOD_SIZE * PERIOD_COUNT / 2;
	cfg.stop_threshold = PERIOD_SIZE * PERIOD_COUNT;
	cfg.silence_threshold = 0;

	pcm = pcm_open(CARD, DEVICE, PCM_OUT, &cfg);
	if (!pcm || !pcm_is_ready(pcm)) {
		fprintf(stderr, "beep: pcm_open C%dD%dp: %s\n", CARD, DEVICE,
			pcm ? pcm_get_error(pcm) : "null");
		goto out_mix;
	}
	amp_during_pcm(m);

	period = pcm_frames_to_bytes(pcm, PERIOD_SIZE);
	buf = malloc(period);
	if (!buf) {
		fprintf(stderr, "beep: oom\n");
		goto out_pcm;
	}

	frames = (RATE / 1000) * BEEP_MS;
	n = 0;
	while (n < frames) {
		unsigned chunk = PERIOD_SIZE;
		unsigned k;
		if (chunk > frames - n)
			chunk = frames - n;
		for (k = 0; k < chunk; k++) {
			/* 880 Hz square-ish: 48000/880 ≈ 54 samples/period */
			short s = (((n + k) / (RATE / TONE_HZ / 2)) & 1) ? TONE_AMP : -TONE_AMP;
			buf[k * 2] = s;
			buf[k * 2 + 1] = s;
		}
		{
			int w, tries = 0;
			while ((w = pcm_writei(pcm, buf, chunk)) != (int)chunk) {
				fprintf(stderr, "beep: pcm_writei: %s\n", pcm_get_error(pcm));
				if (++tries >= 3)
					goto out_buf;
				set_int(m, "Power Up(1:Up_0:Down)", 1);
			}
		}
		n += chunk;
	}
	set_int(m, "Power Up(1:Up_0:Down)", 1);
	err = 0;
	fprintf(stderr, "beep: ok rdma1 sifs1 tonegen %u frames\n", frames);

out_buf:
	free(buf);
out_pcm:
	pcm_close(pcm);
out_mix:
	mixer_close(m);
	return err;
}
