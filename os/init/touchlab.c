/* SaaiOS /sbin/touchlab — static libc, queued TD4150 lab via sysfs.
 * Write /sys/kernel/saaios_touch/action, poll status (sysfs will not poll).
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/input.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/reboot.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

#define EXIT_DEAD 75
#define POLL_US 50000
#define DEFAULT_WINDOW_SEC 10
#define TIMEOUT_SEC 30

struct status {
	unsigned seq;
	char state[16];
	unsigned action;
	int retval;
	unsigned response;
	int live20;
	unsigned mode;
	int attn;
	unsigned irq;
	unsigned rx;
	unsigned report_touch;
};

static volatile sig_atomic_t g_stop;

static void on_sigint(int sig)
{
	(void)sig;
	g_stop = 1;
}

static const char *action_path(void)
{
	static const char *p;
	struct stat st;

	if (p)
		return p;
	if (stat("/sys/kernel/saaios_touch/action", &st) == 0)
		p = "/sys/kernel/saaios_touch/action";
	else if (stat("/sys/class/sec/tsp/saaios_touch/action", &st) == 0)
		p = "/sys/class/sec/tsp/saaios_touch/action";
	else
		p = "/sys/class/sec/tsp/saaios_action";
	return p;
}

static const char *status_path(void)
{
	static const char *p;
	struct stat st;

	if (p)
		return p;
	if (stat("/sys/kernel/saaios_touch/status", &st) == 0)
		p = "/sys/kernel/saaios_touch/status";
	else if (stat("/sys/class/sec/tsp/saaios_touch/status", &st) == 0)
		p = "/sys/class/sec/tsp/saaios_touch/status";
	else
		p = "/sys/class/sec/tsp/saaios_status";
	return p;
}

/* Match key at start of a token (buf start or after whitespace). Kernel
 * status is two lines: seq= state= action= retval= response= / live20=
 * mode= attn= irq= rx= report_touch=. Do not invent dead.
 */
static const char *find_key(const char *buf, const char *key)
{
	const char *p = buf;
	size_t klen = strlen(key);

	if (!buf || !key)
		return NULL;
	while ((p = strstr(p, key)) != NULL) {
		if (p == buf || p[-1] == ' ' || p[-1] == '\n' || p[-1] == '\t' ||
		    p[-1] == '\r')
			return p + klen;
		p += klen;
	}
	return NULL;
}

static int parse_int_at(const char *s, int hex)
{
	long v;
	char *end;

	if (!s)
		return 0;
	while (*s == ' ' || *s == '\t')
		s++;
	v = strtol(s, &end, hex ? 16 : 10);
	(void)end;
	return (int)v;
}

static unsigned parse_uint_at(const char *s, int hex)
{
	unsigned long v;
	char *end;

	if (!s)
		return 0;
	while (*s == ' ' || *s == '\t')
		s++;
	v = strtoul(s, &end, hex ? 16 : 10);
	(void)end;
	return (unsigned)v;
}

static void copy_tok(char *d, size_t cap, const char *s)
{
	size_t i = 0;

	if (!cap)
		return;
	if (!s) {
		d[0] = 0;
		return;
	}
	while (s[i] && s[i] != ' ' && s[i] != '\n' && s[i] != '\t' &&
	       s[i] != '\r' && i + 1 < cap) {
		d[i] = s[i];
		i++;
	}
	d[i] = 0;
}

static char g_raw_status[512];

static int read_status(struct status *st)
{
	const char *v;
	FILE *f;
	size_t n;

	memset(st, 0, sizeof(*st));
	copy_tok(st->state, sizeof(st->state), "?");
	g_raw_status[0] = 0;
	f = fopen(status_path(), "r");
	if (!f)
		return -1;
	n = fread(g_raw_status, 1, sizeof(g_raw_status) - 1, f);
	fclose(f);
	if (n == 0)
		return -1;
	g_raw_status[n] = 0;
	st->seq = parse_uint_at(find_key(g_raw_status, "seq="), 0);
	v = find_key(g_raw_status, "state=");
	if (v)
		copy_tok(st->state, sizeof(st->state), v);
	v = find_key(g_raw_status, "action=0x");
	if (v)
		st->action = parse_uint_at(v, 1);
	else
		st->action = parse_uint_at(find_key(g_raw_status, "action="), 16);
	st->retval = parse_int_at(find_key(g_raw_status, "retval="), 0);
	st->response = parse_uint_at(find_key(g_raw_status, "response="), 16);
	/* live20=0 (never sent) is not dead. Only kernel state=dead is dead. */
	v = find_key(g_raw_status, "live20=");
	st->live20 = v ? parse_int_at(v, 0) : 0;
	st->mode = parse_uint_at(find_key(g_raw_status, "mode="), 16);
	st->attn = parse_int_at(find_key(g_raw_status, "attn="), 0);
	st->irq = parse_uint_at(find_key(g_raw_status, "irq="), 0);
	st->rx = parse_uint_at(find_key(g_raw_status, "rx="), 0);
	st->report_touch = parse_uint_at(find_key(g_raw_status, "report_touch="), 0);
	return 0;
}

static void print_kernel_status(void)
{
	fputs(g_raw_status, stdout);
	if (g_raw_status[0] && g_raw_status[strlen(g_raw_status) - 1] != '\n')
		fputc('\n', stdout);
	fflush(stdout);
}

static const char *token_for_cmd(unsigned cmd)
{
	switch (cmd) {
	case 0x14:
		return "run_app";
	case 0x05:
		return "enable_report";
	case 0x30:
		return "app_config";
	case 0x24:
		return "no_doze";
	case 0x20:
		return "live20";
	default:
		return "status";
	}
}

static void print_json(const struct status *st, const char *action,
		unsigned touch_events)
{
	printf("{\"seq\":%u,\"action\":\"%s\",\"retval\":%d,\"response\":%u,"
	       "\"live20\":%d,\"mode\":%u,\"irq\":%u,\"touch_events\":%u,"
	       "\"state\":\"%s\"}\n",
		st->seq, action ? action : token_for_cmd(st->action),
		st->retval, st->response, st->live20, st->mode, st->irq,
		touch_events, st->state);
	fflush(stdout);
}

static int write_action(const char *token)
{
	int fd;
	char buf[64];
	ssize_t n, w;
	const char *path = action_path();

	n = snprintf(buf, sizeof(buf), "%s\n", token);
	if (n < 0 || (size_t)n >= sizeof(buf))
		return -1;
	fd = open(path, O_WRONLY | O_CLOEXEC);
	if (fd < 0) {
		fprintf(stderr, "touchlab: open %s: %s\n", path, strerror(errno));
		return -1;
	}
	w = write(fd, buf, (size_t)n);
	close(fd);
	if (w < 0) {
		fprintf(stderr, "touchlab: write %s: %s\n", path, strerror(errno));
		return -1;
	}
	return 0;
}

static int wait_not_busy(struct status *st, int timeout_sec)
{
	int waited_ms = 0;
	int limit_ms = timeout_sec * 1000;

	while (waited_ms < limit_ms) {
		if (read_status(st) < 0)
			return -1;
		if (strcmp(st->state, "busy") != 0)
			return 0;
		usleep(POLL_US);
		waited_ms += POLL_US / 1000;
	}
	fprintf(stderr, "touchlab: timeout waiting for status (still busy)\n");
	return -1;
}

static int run_token(const char *token, struct status *st)
{
	int timeout = TIMEOUT_SEC;

	if (write_action(token) < 0) {
		if (read_status(st) == 0)
			print_kernel_status();
		return -1;
	}
	return wait_not_busy(st, timeout);
}

static int is_dead(const struct status *st)
{
	/* Kernel state=dead, or any completed experiment with retval<0. */
	if (strcmp(st->state, "dead") == 0)
		return 1;
	if (st->seq > 0 && st->retval < 0)
		return 1;
	return 0;
}

static void maybe_reboot_and_die(const struct status *st, const char *action,
		unsigned touch_events, int reboot_on_dead)
{
	print_kernel_status();
	print_json(st, action, touch_events);
	if (reboot_on_dead) {
		fprintf(stderr, "touchlab: --reboot-on-dead\n");
		sync();
		reboot(RB_AUTOBOOT);
	}
	exit(EXIT_DEAD);
}

static int open_sec_touchscreen(char *devpath, size_t cap)
{
	int i;
	FILE *f;
	char namepath[80], name[64];
	size_t n;

	for (i = 0; i < 32; i++) {
		snprintf(namepath, sizeof(namepath),
			"/sys/class/input/event%d/device/name", i);
		f = fopen(namepath, "r");
		if (!f)
			continue;
		if (!fgets(name, sizeof(name), f)) {
			fclose(f);
			continue;
		}
		fclose(f);
		n = strlen(name);
		while (n && (name[n - 1] == '\n' || name[n - 1] == '\r'))
			name[--n] = 0;
		if (strcmp(name, "sec_touchscreen") != 0)
			continue;
		snprintf(devpath, cap, "/dev/input/event%d", i);
		return open(devpath, O_RDONLY | O_NONBLOCK | O_CLOEXEC);
	}
	if (cap)
		devpath[0] = 0;
	return -1;
}

static unsigned count_touch_kinds(const struct input_event *ev)
{
	if (ev->type == EV_ABS)
		return 1;
	if (ev->type == EV_SYN)
		return 1;
	if (ev->type == EV_KEY && ev->code == BTN_TOUCH)
		return 1;
	return 0;
}

static unsigned touch_window(int sec, unsigned *irq_before, unsigned *irq_after)
{
	struct status st;
	struct pollfd pfd;
	struct input_event ev;
	struct timespec now, end;
	char devpath[64];
	int fd = -1;
	unsigned n = 0;
	int timeout_ms;
	long remain_ms;

	if (read_status(&st) == 0)
		*irq_before = st.irq;
	else
		*irq_before = 0;
	*irq_after = *irq_before;

	clock_gettime(CLOCK_MONOTONIC, &end);
	end.tv_sec += sec;

	fd = open_sec_touchscreen(devpath, sizeof(devpath));
	if (fd >= 0)
		fprintf(stderr, "touchlab: touch window %ds on %s — touch the screen\n",
			sec, devpath);
	else
		fprintf(stderr, "touchlab: touch window %ds (no sec_touchscreen yet) — touch the screen\n",
			sec);

	while (!g_stop) {
		clock_gettime(CLOCK_MONOTONIC, &now);
		remain_ms = (end.tv_sec - now.tv_sec) * 1000L +
			(end.tv_nsec - now.tv_nsec) / 1000000L;
		if (remain_ms <= 0)
			break;
		if (fd < 0) {
			fd = open_sec_touchscreen(devpath, sizeof(devpath));
			if (fd < 0) {
				usleep(POLL_US);
				continue;
			}
			fprintf(stderr, "touchlab: opened %s\n", devpath);
		}
		pfd.fd = fd;
		pfd.events = POLLIN;
		pfd.revents = 0;
		timeout_ms = remain_ms > 50 ? 50 : (int)remain_ms;
		if (poll(&pfd, 1, timeout_ms) > 0 && (pfd.revents & POLLIN)) {
			ssize_t r = read(fd, &ev, sizeof(ev));
			if (r == (ssize_t)sizeof(ev))
				n += count_touch_kinds(&ev);
			else if (r < 0 && errno != EAGAIN && errno != EINTR)
				break;
		}
	}
	if (fd >= 0)
		close(fd);
	if (read_status(&st) == 0)
		*irq_after = st.irq;
	return n;
}

static int cmd_status(void)
{
	struct status st;

	if (read_status(&st) < 0) {
		fprintf(stderr, "touchlab: cannot read %s\n", status_path());
		return 1;
	}
	print_kernel_status();
	print_json(&st, "status", 0);
	if (is_dead(&st))
		return EXIT_DEAD;
	return 0;
}

static int cmd_run(const char *token)
{
	struct status st;
	int timeout = TIMEOUT_SEC;
	int wr;

	if (strcmp(token, "live20") && strcmp(token, "identify") &&
	    strcmp(token, "run_app") && strcmp(token, "enable_report") &&
	    strcmp(token, "no_doze") && strcmp(token, "app_config")) {
		fprintf(stderr, "touchlab: unknown action '%s'\n", token);
		return 2;
	}
	/* Always write sysfs. Kernel EBUSY if state=dead. */
	wr = write_action(token);
	if (wr < 0) {
		if (read_status(&st) == 0) {
			print_kernel_status();
			print_json(&st, token, 0);
			if (is_dead(&st))
				return EXIT_DEAD;
		}
		return 1;
	}
	if (wait_not_busy(&st, timeout) < 0)
		return 1;
	print_kernel_status();
	print_json(&st, token, 0);
	if (is_dead(&st))
		return EXIT_DEAD;
	return 0;
}

static int cmd_run_all(int window_sec, int reboot_on_dead)
{
	struct status st;
	unsigned irq_b, irq_a, touches;

	if (read_status(&st) < 0) {
		fprintf(stderr, "touchlab: cannot read %s\n", status_path());
		return 1;
	}
	print_kernel_status();
	print_json(&st, "status", 0);
	if (is_dead(&st))
		maybe_reboot_and_die(&st, "status", 0, reboot_on_dead);

	/* Prefer empty live20 first. No app_config (late 0x30 LIVE -62). */
	if (run_token("live20", &st) < 0)
		return 1;
	if (is_dead(&st))
		maybe_reboot_and_die(&st, "live20", 0, reboot_on_dead);
	touches = touch_window(window_sec, &irq_b, &irq_a);
	if (read_status(&st) < 0)
		return 1;
	print_kernel_status();
	print_json(&st, "live20", touches);
	if (is_dead(&st))
		maybe_reboot_and_die(&st, "live20", touches, reboot_on_dead);

	if (!is_dead(&st)) {
		if (run_token("run_app", &st) < 0)
			return 1;
		if (is_dead(&st))
			maybe_reboot_and_die(&st, "run_app", 0, reboot_on_dead);
		touches = touch_window(window_sec, &irq_b, &irq_a);
		if (read_status(&st) < 0)
			return 1;
		print_kernel_status();
		print_json(&st, "run_app", touches);
		if (is_dead(&st))
			maybe_reboot_and_die(&st, "run_app", touches, reboot_on_dead);
	}

	if (!is_dead(&st)) {
		if (run_token("enable_report", &st) < 0)
			return 1;
		if (is_dead(&st))
			maybe_reboot_and_die(&st, "enable_report", 0, reboot_on_dead);
		touches = touch_window(window_sec, &irq_b, &irq_a);
		if (read_status(&st) < 0)
			return 1;
		print_kernel_status();
		print_json(&st, "enable_report", touches);
		if (is_dead(&st))
			maybe_reboot_and_die(&st, "enable_report", touches,
				reboot_on_dead);
	}

	if (!is_dead(&st)) {
		if (run_token("no_doze", &st) < 0)
			return 1;
		if (is_dead(&st))
			maybe_reboot_and_die(&st, "no_doze", 0, reboot_on_dead);
		touches = touch_window(window_sec, &irq_b, &irq_a);
		if (read_status(&st) < 0)
			return 1;
		print_kernel_status();
		print_json(&st, "no_doze", touches);
		if (is_dead(&st))
			maybe_reboot_and_die(&st, "no_doze", touches,
				reboot_on_dead);
	}

	return 0;
}

static int cmd_monitor(void)
{
	struct status st;
	struct pollfd pfd;
	struct input_event ev;
	char devpath[64];
	int fd = -1;
	unsigned last_seq = (unsigned)-1;

	signal(SIGINT, on_sigint);
	signal(SIGTERM, on_sigint);

	while (!g_stop) {
		if (read_status(&st) == 0 && st.seq != last_seq) {
			print_kernel_status();
			print_json(&st, token_for_cmd(st.action), 0);
			last_seq = st.seq;
		} else if (read_status(&st) == 0) {
			print_kernel_status();
			printf("status seq=%u state=%s action=0x%02x retval=%d response=%02x live20=%d mode=%02x irq=%u rx=%u report_touch=%u\n",
				st.seq, st.state, st.action, st.retval, st.response,
				st.live20, st.mode, st.irq, st.rx, st.report_touch);
			fflush(stdout);
		}
		if (fd < 0)
			fd = open_sec_touchscreen(devpath, sizeof(devpath));
		if (fd < 0) {
			usleep(200000);
			continue;
		}
		pfd.fd = fd;
		pfd.events = POLLIN;
		pfd.revents = 0;
		if (poll(&pfd, 1, 500) > 0 && (pfd.revents & POLLIN)) {
			ssize_t r = read(fd, &ev, sizeof(ev));
			if (r == (ssize_t)sizeof(ev)) {
				printf("event type=%u code=%u value=%d\n",
					ev.type, ev.code, ev.value);
				fflush(stdout);
			} else if (r < 0 && errno != EAGAIN && errno != EINTR) {
				close(fd);
				fd = -1;
			}
		}
	}
	if (fd >= 0)
		close(fd);
	return 0;
}

static void usage(void)
{
	fprintf(stderr,
		"usage: touchlab status\n"
		"       touchlab run live20|run_app|enable_report|no_doze|identify\n"
		"       touchlab run-all [--touch-window SEC] [--reboot-on-dead]\n"
		"       touchlab monitor\n");
}

int main(int argc, char **argv)
{
	int window_sec = DEFAULT_WINDOW_SEC;
	int reboot_on_dead = 0;
	int i;

	if (argc < 2) {
		usage();
		return 2;
	}
	if (strcmp(argv[1], "status") == 0)
		return cmd_status();
	if (strcmp(argv[1], "run") == 0) {
		if (argc < 3) {
			usage();
			return 2;
		}
		return cmd_run(argv[2]);
	}
	if (strcmp(argv[1], "run-all") == 0) {
		signal(SIGINT, on_sigint);
		for (i = 2; i < argc; i++) {
			if (strcmp(argv[i], "--touch-window") == 0 && i + 1 < argc) {
				window_sec = atoi(argv[++i]);
				if (window_sec <= 0)
					window_sec = DEFAULT_WINDOW_SEC;
			} else if (strcmp(argv[i], "--reboot-on-dead") == 0) {
				reboot_on_dead = 1;
			} else {
				fprintf(stderr, "touchlab: unknown arg '%s'\n", argv[i]);
				usage();
				return 2;
			}
		}
		return cmd_run_all(window_sec, reboot_on_dead);
	}
	if (strcmp(argv[1], "monitor") == 0)
		return cmd_monitor();
	usage();
	return 2;
}
