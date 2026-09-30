/* Direct AP-phys RO (optional guarded poke). No ATU.
 *
 * System RAM (cp_rmem*): pass memremap/memunmap from kallsyms — GKI does
 * not export them to modules. PCIe BAR / RC windows: plain ioremap(_wc).
 */
#include <linux/module.h>
#include <linux/io.h>
#include <linux/kstrtox.h>
#include <linux/errno.h>

#define SAAIOS_PHYS_MAP_MAX	SZ_2M
#define MEMREMAP_WB		(1UL << 0)
#define MEMREMAP_WT		(1UL << 1)
#define MEMREMAP_WC		(1UL << 2)

typedef void *(*memremap_fn_t)(resource_size_t offset, size_t size,
			       unsigned long flags);
typedef void (*memunmap_fn_t)(void *addr);

static char phys_hex[24] = "0";
module_param_string(phys, phys_hex, sizeof(phys_hex), 0644);

static char map_size_hex[24] = "0x100000";
module_param_string(map_size, map_size_hex, sizeof(map_size_hex), 0644);

static unsigned long dump_off;
module_param(dump_off, ulong, 0644);

static int dump_n = 32;
module_param(dump_n, int, 0644);

static int read_only = 1;
module_param(read_only, int, 0644);

static int poke_val = -1;
module_param(poke_val, int, 0644);

static char expect_hex[24] = "";
module_param_string(expect_hex, expect_hex, sizeof(expect_hex), 0644);

static char memremap_fn_hex[24] = "0";
module_param_string(memremap_fn, memremap_fn_hex, sizeof(memremap_fn_hex), 0644);

static char memunmap_fn_hex[24] = "0";
module_param_string(memunmap_fn, memunmap_fn_hex, sizeof(memunmap_fn_hex), 0644);

static int dry_run = 1;
module_param(dry_run, int, 0644);

static int parse_expect(u8 *out, int max)
{
	int n = 0;
	const char *p = expect_hex;

	if (!p || !p[0])
		return 0;
	while (*p && n < max) {
		char b[3] = { 0 };
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

static void unmap_any(void *v, int used_memremap, memunmap_fn_t munmap_fn)
{
	if (!v)
		return;
	if (used_memremap && munmap_fn)
		munmap_fn(v);
	else if (!used_memremap)
		iounmap((void __iomem *)v);
}

static int __nocfi do_phys(void)
{
	void *v = NULL;
	unsigned long phys, msz, mr_a, mu_a;
	memremap_fn_t mremap = NULL;
	memunmap_fn_t munmap_fn = NULL;
	int dn, i, hi, exp_len;
	u8 exp[16];
	char hexbuf[3 * 40];
	u8 saw;
	int all_ff = 1;
	int all_00 = 1;
	int used_memremap = 0;

	if (kstrtoul(phys_hex, 0, &phys) || kstrtoul(map_size_hex, 0, &msz))
		return -EINVAL;
	if (!phys || !msz || msz > SAAIOS_PHYS_MAP_MAX)
		return -EINVAL;
	if (dump_off >= msz)
		return -EINVAL;
	exp_len = parse_expect(exp, (int)sizeof(exp));
	if (exp_len < 0)
		return -EINVAL;
	if (!read_only) {
		if (poke_val < 0 || poke_val > 255)
			return -EINVAL;
		if (exp_len < 1) {
			pr_err("saaios_phys_ro: refuse write without expect_hex\n");
			return -EPERM;
		}
	}

	if (!kstrtoul(memremap_fn_hex, 0, &mr_a) && mr_a)
		mremap = (memremap_fn_t)mr_a;
	if (!kstrtoul(memunmap_fn_hex, 0, &mu_a) && mu_a)
		munmap_fn = (memunmap_fn_t)mu_a;

	if (mremap) {
		v = mremap(phys, msz, MEMREMAP_WC);
		if (v) {
			used_memremap = 1;
			pr_info("saaios_phys_ro: memremap_WC ok\n");
		} else {
			v = mremap(phys, msz, MEMREMAP_WB);
			if (v) {
				used_memremap = 1;
				pr_info("saaios_phys_ro: memremap_WB ok\n");
			}
		}
	}
	if (!v) {
		v = (void *)ioremap_wc(phys, msz);
		if (v)
			pr_info("saaios_phys_ro: ioremap_wc ok\n");
	}
	if (!v) {
		v = (void *)ioremap(phys, msz);
		if (v)
			pr_info("saaios_phys_ro: ioremap ok\n");
	}
	if (!v) {
		pr_err("saaios_phys_ro: map FAIL phys=0x%lx size=0x%lx\n",
		       phys, msz);
		return -EFAULT;
	}
	pr_info("saaios_phys_ro: map phys=0x%lx size=0x%lx v=%px off=0x%lx\n",
		phys, msz, v, dump_off);

	dn = dump_n;
	if (dn < 1)
		dn = 1;
	if (dn > 32)
		dn = 32;
	if (dump_off + (unsigned long)dn > msz)
		dn = (int)(msz - dump_off);

	hi = 0;
	for (i = 0; i < dn; i++) {
		u8 b = readb((void __iomem *)v + dump_off + i);

		if (b != 0xff)
			all_ff = 0;
		if (b != 0x00)
			all_00 = 0;
		hi += scnprintf(hexbuf + hi, sizeof(hexbuf) - hi, "%02x%s",
				b, (i + 1 < dn) ? " " : "");
	}
	pr_info("saaios_phys_ro: DUMP @0x%lx+0x%lx n=%d : %s\n",
		phys, dump_off, dn, hexbuf);
	if (all_ff)
		pr_info("saaios_phys_ro: VERDICT all-ff (open-bus/empty)\n");
	else if (all_00)
		pr_info("saaios_phys_ro: VERDICT all-00\n");
	else
		pr_info("saaios_phys_ro: VERDICT mixed (possible real data)\n");

	if (exp_len > 0) {
		int ok = 1;

		if (exp_len > dn)
			ok = 0;
		for (i = 0; ok && i < exp_len; i++) {
			if (readb((void __iomem *)v + dump_off + i) != exp[i])
				ok = 0;
		}
		pr_info("saaios_phys_ro: EXPECT %s\n", ok ? "MATCH" : "MISS");
		if (!read_only && !ok) {
			pr_err("saaios_phys_ro: no write — expect miss\n");
			unmap_any(v, used_memremap, munmap_fn);
			return -EIO;
		}
	}

	if (!read_only) {
		writeb((u8)poke_val, (void __iomem *)v + dump_off);
		wmb();
		saw = readb((void __iomem *)v + dump_off);
		pr_info("saaios_phys_ro: WRITE 0x%02x @0x%lx+0x%lx now=0x%02x\n",
			poke_val, phys, dump_off, saw);
	}

	unmap_any(v, used_memremap, munmap_fn);
	return 0;
}

static int __init saaios_phys_ro_init(void)
{
	pr_info("saaios_phys_ro: init dry=%d ro=%d phys=%s map=%s off=0x%lx\n",
		dry_run, read_only, phys_hex, map_size_hex, dump_off);
	if (dry_run) {
		pr_info("saaios_phys_ro: dry_run OK\n");
		return 0;
	}
	return do_phys();
}

static void __exit saaios_phys_ro_exit(void)
{
	pr_info("saaios_phys_ro: exit\n");
}

module_init(saaios_phys_ro_init);
module_exit(saaios_phys_ro_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("SaaiOS direct phys RO/poke (no ATU)");
MODULE_AUTHOR("SaaiOS");
