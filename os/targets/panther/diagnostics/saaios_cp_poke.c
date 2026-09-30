/* CP DRAM poke via modem PCIe outbound ATU (s51xx / ch0 / 11920000.pcie).
 *
 * Live panther evidence (NOT wifi 14520000):
 *   dmesg: "PCIe Channel Number:0" + s51xx on 0000:01:00.0
 *   DT pcie@11920000 ch-num=0 ep-device-type=4 (modem)
 *   DT pcie@14520000 ch-num=1 ep-device-type=1 (wlan)
 *   exynos_pcie_rc_set_outbound_atu: AP base = pci_bus_res.start + 0x200000
 *     → ch0: 0x40000000 + 0x200000 = 0x40200000 (BTL's 0x14200000 is old SoC)
 *
 * Protocol: dry_run=1; then read_only=1; then ONE write poke_val=5 @0x414f47f4.
 */
#include <linux/module.h>
#include <linux/io.h>
#include <linux/delay.h>
#include <linux/kstrtox.h>
#include <linux/errno.h>

#define SAAIOS_POKE_MAP_SIZE	SZ_1M
/* Modem ch0 bus window start + 2MiB — from pcie_exynos_gs set_outbound_atu. */
#define SAAIOS_MODEM_AP_BASE	0x40200000ul
#define SAAIOS_MODEM_PCIE_CH	0

typedef int (*atu_fn_t)(int ch_num, u32 target_addr, u32 offset, u32 size);
typedef void (*l1ss_fn_t)(int enable, int ch_num);
typedef int (*link_fn_t)(int ch_num);

static char atu_fn_hex[24] = "0";
module_param_string(atu_fn, atu_fn_hex, sizeof(atu_fn_hex), 0644);

static char l1ss_fn_hex[24] = "0";
module_param_string(l1ss_fn, l1ss_fn_hex, sizeof(l1ss_fn_hex), 0644);

static char link_fn_hex[24] = "0";
module_param_string(link_fn, link_fn_hex, sizeof(link_fn_hex), 0644);

static int pcie_ch = SAAIOS_MODEM_PCIE_CH;
module_param(pcie_ch, int, 0644);

static char cp_phys_hex[24] = "0";
module_param_string(cp_phys, cp_phys_hex, sizeof(cp_phys_hex), 0644);

static char ap_base_hex[24] = "0x40200000";
module_param_string(ap_base, ap_base_hex, sizeof(ap_base_hex), 0644);

static int poke_val = -1;
module_param(poke_val, int, 0644);

static int dry_run = 1;
module_param(dry_run, int, 0644);

static int read_only;
module_param(read_only, int, 0644);

/* Optional hex signature (even length). If set with read_only: dump 32B at
 * cp_phys, then scan whole 1MiB ATU window for the signature. */
static char sig_hex[96] = "";
module_param_string(sig_hex, sig_hex, sizeof(sig_hex), 0644);

static int dump_n = 32;
module_param(dump_n, int, 0644);

static int parse_sig(u8 *out, int max)
{
	int n = 0;
	const char *p = sig_hex;
	if (!p || !p[0])
		return 0;
	while (*p && n < max) {
		char b[3] = {0};
		unsigned long v;
		while (*p == ' ' || *p == ':' || *p == '-')
			p++;
		if (!p[0] || !p[1])
			break;
		b[0] = p[0];
		b[1] = p[1];
		if (kstrtoul(b, 16, &v))
			return -EINVAL;
		out[n++] = (u8)v;
		p += 2;
	}
	return n;
}

static int __nocfi do_poke(void)
{
	atu_fn_t atu;
	l1ss_fn_t l1ss = NULL;
	link_fn_t link = NULL;
	void __iomem *v;
	u32 atu_off;
	unsigned long atu_fn, cp_phys, ap_base, l1ss_a, link_a;
	int ret, tries;
	u8 saw;
	u8 sig[32];
	int sig_len;
	int dn;
	char hexbuf[3 * 40];
	int i, hi;

	if (kstrtoul(atu_fn_hex, 0, &atu_fn) || kstrtoul(cp_phys_hex, 0, &cp_phys) ||
	    kstrtoul(ap_base_hex, 0, &ap_base))
		return -EINVAL;
	if (!atu_fn || !cp_phys || !ap_base)
		return -EINVAL;
	if (!read_only && (poke_val < 0 || poke_val > 255))
		return -EINVAL;
	if (pcie_ch != SAAIOS_MODEM_PCIE_CH) {
		pr_err("saaios_cp_poke: refuse non-modem ch=%d (modem=0)\n", pcie_ch);
		return -EPERM;
	}

	sig_len = parse_sig(sig, (int)sizeof(sig));
	if (sig_len < 0)
		return -EINVAL;

	atu = (atu_fn_t)atu_fn;
	if (!kstrtoul(l1ss_fn_hex, 0, &l1ss_a) && l1ss_a)
		l1ss = (l1ss_fn_t)l1ss_a;
	if (!kstrtoul(link_fn_hex, 0, &link_a) && link_a)
		link = (link_fn_t)link_a;

	if (l1ss) {
		l1ss(0, pcie_ch);
		pr_info("saaios_cp_poke: L1SS off via s51xx\n");
		usleep_range(3000, 5000);
	}
	if (link) {
		ret = link(pcie_ch);
		pr_info("saaios_cp_poke: link_status=%d (0 may mean L1; continue to ATU)\n",
			ret);
	}

	atu_off = (u32)(cp_phys & ~(SAAIOS_POKE_MAP_SIZE - 1));
	pr_info("saaios_cp_poke: modem ATU ch=%d tgt=0x%x ap=0x%lx ro=%d sig_len=%d\n",
		pcie_ch, atu_off, ap_base, read_only, sig_len);

	ret = -EPIPE;
	for (tries = 0; tries < 8; tries++) {
		if (l1ss && tries)
			l1ss(0, pcie_ch);
		ret = atu(pcie_ch, atu_off, 0, SAAIOS_POKE_MAP_SIZE);
		pr_info("saaios_cp_poke: ATU try=%d ret=%d\n", tries, ret);
		if (ret == 0)
			break;
		usleep_range(10000, 15000);
	}
	if (ret) {
		pr_err("saaios_cp_poke: ATU failed ret=%d — abort, no MMIO\n", ret);
		return ret;
	}

	v = ioremap(ap_base, SAAIOS_POKE_MAP_SIZE);
	if (!v)
		return -ENOMEM;

	saw = readb(v + (cp_phys - atu_off));
	pr_info("saaios_cp_poke: RO @0x%lx = 0x%02x\n", cp_phys, saw);

	dn = dump_n;
	if (dn < 1)
		dn = 1;
	if (dn > 32)
		dn = 32;
	hi = 0;
	for (i = 0; i < dn && (cp_phys - atu_off + i) < SAAIOS_POKE_MAP_SIZE; i++) {
		u8 b = readb(v + (cp_phys - atu_off + i));
		hi += scnprintf(hexbuf + hi, sizeof(hexbuf) - hi, "%02x%s",
				b, (i + 1 < dn) ? " " : "");
	}
	pr_info("saaios_cp_poke: DUMP @0x%lx n=%d : %s\n", cp_phys, dn, hexbuf);

	if (read_only && sig_len > 0) {
		/* Probe fixed offsets inside the 1MiB window (not full memchr —
		 * avoids multi-second bus hammer / SError storms). */
		static const u32 probes[] = {
			0x00000, /* window base / TOC-ish */
			0x10000, 0x16c10 & 0xfffff, /* MAIN file start low bits */
			0xf47f4, /* SET#2 VA low 20 bits: 0x414f47f4 */
			0xf47f6,
			0x00010, 0x00400, 0x08000,
		};
		int found = 0;
		int ff = 0;
		for (i = 0; i < 64; i++)
			if (readb(v + i) == 0xff)
				ff++;
		if (ff == 64) {
			pr_info("saaios_cp_poke: WIN all-ff skip tgt=0x%x\n", atu_off);
		} else {
			unsigned pi;
			for (pi = 0; pi < ARRAY_SIZE(probes); pi++) {
				u32 off = probes[pi];
				int j;
				if (off + (u32)sig_len > SAAIOS_POKE_MAP_SIZE)
					continue;
				for (j = 0; j < sig_len; j++) {
					if (readb(v + off + j) != sig[j])
						break;
				}
				if (j == sig_len) {
					pr_info("saaios_cp_poke: SIG HIT @0x%x (win+0x%x)\n",
						atu_off + off, off);
					found = 1;
					break;
				}
			}
			if (!found)
				pr_info("saaios_cp_poke: SIG MISS tgt=0x%x\n", atu_off);
		}
	}

	if (!read_only) {
		writeb((u8)poke_val, v + (cp_phys - atu_off));
		wmb();
		saw = readb(v + (cp_phys - atu_off));
		pr_info("saaios_cp_poke: wrote 0x%02x @0x%lx now=0x%02x OK\n",
			poke_val, cp_phys, saw);
	}

	iounmap(v);
	return 0;
}

static int __init saaios_cp_poke_init(void)
{
	pr_info("saaios_cp_poke: init dry=%d ro=%d atu=%s phys=%s ap=%s val=%d ch=%d\n",
		dry_run, read_only, atu_fn_hex, cp_phys_hex, ap_base_hex, poke_val, pcie_ch);
	if (dry_run) {
		pr_info("saaios_cp_poke: dry_run OK (modem ch0 / ap 0x40200000)\n");
		return 0;
	}
	return do_poke();
}

static void __exit saaios_cp_poke_exit(void)
{
	pr_info("saaios_cp_poke: exit\n");
}

module_init(saaios_cp_poke_init);
module_exit(saaios_cp_poke_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("SaaiOS modem-RC CP DRAM poke (ch0/11920000)");
MODULE_AUTHOR("SaaiOS");
