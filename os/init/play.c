/* SaaiOS /sbin/play — WAV on SMA1303 via ABOX RDMA1 / SIFS1.
 * Same LIVE v026 route as /sbin/beep: pcmC0D1p → SPUS OUT1 → SIFS1 → UAIF1.
 * Do not open pcmC0D3p / SIFS0. Do not toggle Codec Enable, SMA I2C reset,
 * HP/EP/codec SPK, Force AMP Power Down.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <dirent.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <linux/input.h>
#include <tinyalsa/asoundlib.h>

#define CARD 0
#define DEVICE 1 /* pcmC0D1p RDMA1 — do not use D3 */
#define RATE 48000
#define CHANNELS 2
#define PERIOD_SIZE 960
#define PERIOD_COUNT 4
#define SPK_VOL 160
#define MAX_SRC_BYTES (8u * 1024u * 1024u)

static void set_enum(struct mixer *m, const char *name, const char *val)
{
	struct mixer_ctl *c = mixer_get_ctl_by_name(m, name);
	int r;
	if (!c) {
		fprintf(stderr, "play: missing %s\n", name);
		return;
	}
	r = mixer_ctl_set_enum_by_string(c, val);
	if (r)
		fprintf(stderr, "play: %s=%s failed %d\n", name, val, r);
}

static void set_int(struct mixer *m, const char *name, int v)
{
	struct mixer_ctl *c = mixer_get_ctl_by_name(m, name);
	int r;
	if (!c) {
		fprintf(stderr, "play: missing %s\n", name);
		return;
	}
	r = mixer_ctl_set_value(c, 0, v);
	if (r)
		fprintf(stderr, "play: %s=%d failed %d\n", name, v, r);
}

static void log_jack(void)
{
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
			fprintf(stderr, "play: jack %s hp=%d mic=%d\n", e->d_name,
				!!(sw[0] & (1ul << SW_HEADPHONE_INSERT)),
				!!(sw[0] & (1ul << SW_MICROPHONE_INSERT)));
		close(fd);
		break;
	}
	closedir(d);
}

static void route_speaker(struct mixer *m)
{
	log_jack();
	set_enum(m, "ABOX Sound Type", "SPEAKER");
	set_enum(m, "ABOX UAIF0 SPK", "RESERVED");
	set_int(m, "HP HP On", 0);
	set_int(m, "EP EP On", 0);
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
	set_int(m, "Speaker Mode", 1);
	set_int(m, "Speaker Mute Switch(1:muted_0:un)", 0);
	set_int(m, "Power Up(1:Up_0:Down)", 1);
	set_int(m, "ABOX TONEGEN_1KHZ", 1);
	set_enum(m, "ABOX RDMA1_A", "TONEGEN_1KHZ");
	set_enum(m, "ABOX UAIF1 SPK", "SIFS1");
}

static uint16_t u16le(const unsigned char *p)
{
	return (uint16_t)p[0] | ((uint16_t)p[1] << 8);
}

static uint32_t u32le(const unsigned char *p)
{
	return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
	       ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static int read_fully(FILE *f, void *buf, size_t n)
{
	return fread(buf, 1, n, f) == n;
}

/* Load PCM WAV to 48 kHz S16 stereo. */
static int load_wav(const char *path, short **out, unsigned *out_frames)
{
	FILE *f;
	unsigned char hdr[12], chunk[8], fmt[40];
	uint16_t audio_fmt = 0, ch = 0, bits = 0, blk = 0;
	uint32_t src_rate = 0, data_bytes = 0;
	long data_off = -1;
	unsigned src_frames, dst_frames, i;
	short *src = NULL, *dst = NULL;
	int err = -1;

	f = fopen(path, "rb");
	if (!f) {
		perror("play: open");
		return -1;
	}
	if (!read_fully(f, hdr, 12) || memcmp(hdr, "RIFF", 4) || memcmp(hdr + 8, "WAVE", 4)) {
		fprintf(stderr, "play: not a WAV (need RIFF/WAVE)\n");
		goto out;
	}
	while (read_fully(f, chunk, 8)) {
		uint32_t csz = u32le(chunk + 4);
		if (!memcmp(chunk, "fmt ", 4)) {
			if (csz < 16 || csz > sizeof(fmt) || !read_fully(f, fmt, csz)) {
				fprintf(stderr, "play: bad fmt chunk\n");
				goto out;
			}
			audio_fmt = u16le(fmt);
			ch = u16le(fmt + 2);
			src_rate = u32le(fmt + 4);
			blk = u16le(fmt + 12);
			bits = u16le(fmt + 14);
			if (csz & 1)
				fseek(f, 1, SEEK_CUR);
			continue;
		}
		if (!memcmp(chunk, "data", 4)) {
			data_bytes = csz;
			data_off = ftell(f);
			fseek(f, (long)csz + (csz & 1), SEEK_CUR);
			continue;
		}
		fseek(f, (long)csz + (csz & 1), SEEK_CUR);
	}
	if (data_off < 0 || !src_rate || !ch || !bits) {
		fprintf(stderr, "play: missing fmt/data\n");
		goto out;
	}
	if (audio_fmt != 1 && audio_fmt != 0xfffe) {
		fprintf(stderr, "play: WAV format %u not PCM\n", audio_fmt);
		goto out;
	}
	if (bits != 16 || ch < 1 || ch > 2 || !blk) {
		fprintf(stderr, "play: need 16-bit mono/stereo PCM (got %u-bit %u ch)\n",
			bits, ch);
		goto out;
	}
	if (data_bytes > MAX_SRC_BYTES) {
		fprintf(stderr, "play: file too large (%u bytes, max %u)\n",
			data_bytes, MAX_SRC_BYTES);
		goto out;
	}
	src_frames = data_bytes / blk;
	if (!src_frames) {
		fprintf(stderr, "play: empty data\n");
		goto out;
	}
	src = malloc((size_t)src_frames * ch * sizeof(short));
	if (!src) {
		fprintf(stderr, "play: oom src\n");
		goto out;
	}
	if (fseek(f, data_off, SEEK_SET) != 0 ||
	    !read_fully(f, src, (size_t)src_frames * blk)) {
		fprintf(stderr, "play: short data read\n");
		goto out;
	}
	dst_frames = (unsigned)(((uint64_t)src_frames * RATE + src_rate / 2) / src_rate);
	if (!dst_frames)
		dst_frames = 1;
	dst = malloc((size_t)dst_frames * CHANNELS * sizeof(short));
	if (!dst) {
		fprintf(stderr, "play: oom dst\n");
		goto out;
	}
	for (i = 0; i < dst_frames; i++) {
		uint64_t num = (uint64_t)i * src_rate;
		unsigned si = (unsigned)(num / RATE);
		unsigned frac = (unsigned)(num % RATE);
		unsigned sj = si + 1;
		short l0, r0, l1, r1;
		int l, r;

		if (si >= src_frames)
			si = src_frames - 1;
		if (sj >= src_frames)
			sj = src_frames - 1;
		if (ch == 1) {
			l0 = r0 = src[si];
			l1 = r1 = src[sj];
		} else {
			l0 = src[si * 2];
			r0 = src[si * 2 + 1];
			l1 = src[sj * 2];
			r1 = src[sj * 2 + 1];
		}
		l = ((int)l0 * (int)(RATE - frac) + (int)l1 * (int)frac) / RATE;
		r = ((int)r0 * (int)(RATE - frac) + (int)r1 * (int)frac) / RATE;
		dst[i * 2] = (short)l;
		dst[i * 2 + 1] = (short)r;
	}
	*out = dst;
	*out_frames = dst_frames;
	dst = NULL;
	err = 0;
	fprintf(stderr, "play: wav %u Hz %u ch -> 48000 stereo %u frames\n",
		src_rate, ch, dst_frames);
out:
	free(src);
	free(dst);
	fclose(f);
	return err;
}

static int write_pcm(struct pcm *pcm, struct mixer *m, const short *stereo, unsigned frames)
{
	unsigned n = 0;

	while (n < frames) {
		unsigned chunk = PERIOD_SIZE;
		unsigned off = 0;
		int stalls = 0;

		if (chunk > frames - n)
			chunk = frames - n;
		/* Advance by frames actually written; a partial pcm_writei
		 * must not resend already-played frames from offset 0.
		 */
		while (off < chunk) {
			int w = pcm_writei(pcm, stereo + (n + off) * 2, chunk - off);
			if (w > 0) {
				off += (unsigned)w;
				stalls = 0;
				continue;
			}
			fprintf(stderr, "play: pcm_writei: %s\n", pcm_get_error(pcm));
			if (++stalls >= 3)
				return -1;
			set_int(m, "Power Up(1:Up_0:Down)", 1);
		}
		n += chunk;
	}
	set_int(m, "Power Up(1:Up_0:Down)", 1);
	return 0;
}

int main(int argc, char **argv)
{
	struct mixer *m;
	struct pcm *pcm;
	struct pcm_config cfg;
	short *pcm48 = NULL;
	unsigned frames = 0;
	int err = 1;
	const char *path;

	if (argc != 2 || !argv[1][0] || argv[1][0] == '-') {
		fprintf(stderr, "usage: play FILE.wav\n");
		fprintf(stderr, "  packed clip: /usr/share/sounds/test.wav\n");
		return 2;
	}
	path = argv[1];
	if (load_wav(path, &pcm48, &frames))
		return 1;

	m = mixer_open(CARD);
	if (!m) {
		fprintf(stderr, "play: mixer_open card%d failed\n", CARD);
		goto out_buf;
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
		fprintf(stderr, "play: pcm_open C%dD%dp: %s\n", CARD, DEVICE,
			pcm ? pcm_get_error(pcm) : "null");
		goto out_mix;
	}
	amp_during_pcm(m);

	if (write_pcm(pcm, m, pcm48, frames))
		goto out_pcm;
	err = 0;
	fprintf(stderr, "play: ok rdma1 sifs1 %u frames %s\n", frames, path);

out_pcm:
	pcm_close(pcm);
out_mix:
	mixer_close(m);
out_buf:
	free(pcm48);
	return err;
}
