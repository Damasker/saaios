/* RO: SHMEM_IPC + optional BTL ATU windows for live SIM layout (pin1=2).
 * Factory cites (MODEM-RUNTIME):
 *   SIT 0x0200 body: card@0 apps@2 type@3 state@5 pin1@60
 *   STATUS base: +0xBF4=app_state, +0xBF5=Pin1Verified, +0xBF6=Present
 *   PresentObj getobj size #0x636c; FirstPIN Pin1Verified @obj+20
 * No writes.
 */
#include <linux/module.h>
#include <linux/io.h>
#include <linux/delay.h>
#include <linux/kstrtox.h>

typedef void __iomem *(*get_region_t)(u32 cp, u32 idx);
typedef int (*atu_fn_t)(int ch_num, u32 target_addr, u32 offset, u32 size);

static char region_fn_hex[24] = "0";
module_param_string(region_fn, region_fn_hex, sizeof(region_fn_hex), 0644);

static char atu_fn_hex[24] = "0";
module_param_string(atu_fn, atu_fn_hex, sizeof(atu_fn_hex), 0644);

static int ipc_idx = 3;
module_param(ipc_idx, int, 0644);

static int do_btl = 1;
module_param(do_btl, int, 0644);

static int scan_wire(void __iomem *base, u32 off0, u32 len, const char *tag)
{
	u32 off;
	int hits = 0;

	for (off = 0; off + 64 < len; off++) {
		u8 card = readb(base + off0 + off);
		u8 apps = readb(base + off0 + off + 2);
		u8 type = readb(base + off0 + off + 3);
		u8 state = readb(base + off0 + off + 5);
		u8 pin1 = readb(base + off0 + off + 60);
		/* live soft-lock: PRESENT/USIM/PIN + pin1 ENABLED_VERIFIED(2) */
		if (card != 1 || apps != 1 || type != 2)
			continue;
		if (state != 2 && state != 1 && state != 5)
			continue;
		if (pin1 != 2 && pin1 != 3 && pin1 != 1)
			continue;
		pr_info("saaios_sim_ro: WIRE %s off=0x%x app=%u pin1=%u\n",
			tag, off0 + off, state, pin1);
		hits++;
		if (hits >= 12)
			break;
	}
	pr_info("saaios_sim_ro: WIRE %s hits=%d\n", tag, hits);
	return hits;
}

/* STATUS heap: +0xBF4 app=2, +0xBF5 Pin1Verified=1 (after VerifyPin). */
static int scan_status(void __iomem *base, u32 off0, u32 len, const char *tag)
{
	u32 off;
	int hits = 0;

	if (len < 0xC00)
		return 0;
	for (off = 0; off + 0xBF7 < len; off++) {
		u8 app = readb(base + off0 + off + 0xBF4);
		u8 p1v = readb(base + off0 + off + 0xBF5);
		u8 present = readb(base + off0 + off + 0xBF6);
		if (app != 2)
			continue;
		if (p1v != 1 && p1v != 0)
			continue;
		/* Pin1Verified live should be 1; still log p1v=0 candidates sparingly */
		if (p1v == 0 && (off & 0xFF) != 0)
			continue;
		pr_info("saaios_sim_ro: STATUS %s base=0x%x app=%u p1v=%u present=%u\n",
			tag, off0 + off, app, p1v, present);
		hits++;
		if (hits >= 8)
			break;
	}
	pr_info("saaios_sim_ro: STATUS %s hits=%d\n", tag, hits);
	return hits;
}

static int non_ff_count(void __iomem *v, u32 n)
{
	u32 i, c = 0;
	for (i = 0; i < n; i++)
		if (readb(v + i) != 0xff)
			c++;
	return (int)c;
}

static int scan_btl(void)
{
	atu_fn_t atu;
	unsigned long atu_fn;
	void __iomem *v;
	int total = 0;
	int wi;
	/* Factory BTL CP 0x47200000 → AP ATU 0x87200000; sample first 8 MiB. */
	const u32 btl_ap = 0x87200000u;
	const unsigned long ap_win = 0x40200000ul;

	if (kstrtoul(atu_fn_hex, 0, &atu_fn) || !atu_fn) {
		pr_info("saaios_sim_ro: BTL skip (no atu_fn)\n");
		return 0;
	}
	atu = (atu_fn_t)atu_fn;

	for (wi = 0; wi < 8; wi++) {
		u32 tgt = btl_ap + (u32)wi * SZ_1M;
		int ret, tries;
		int live;

		ret = -EPIPE;
		for (tries = 0; tries < 4; tries++) {
			ret = atu(0, tgt, 0, SZ_1M);
			if (ret == 0)
				break;
			usleep_range(5000, 8000);
		}
		if (ret) {
			pr_info("saaios_sim_ro: BTL ATU fail win=%d tgt=0x%x ret=%d\n",
				wi, tgt, ret);
			continue;
		}
		v = ioremap(ap_win, SZ_1M);
		if (!v)
			return total;
		live = non_ff_count(v, 256);
		pr_info("saaios_sim_ro: BTL win=%d tgt=0x%x nonff256=%d\n", wi, tgt, live);
		if (live > 8) {
			total += scan_wire(v, 0, SZ_1M, "btl");
			total += scan_status(v, 0, SZ_1M, "btl");
		} else {
			pr_info("saaios_sim_ro: BTL win=%d empty/ff — no SIM scan\n", wi);
		}
		iounmap(v);
	}
	return total;
}

static int __init saaios_sim_ro_init(void)
{
	get_region_t get_region;
	unsigned long fn;
	void __iomem *base;
	int total = 0;

	pr_info("saaios_sim_ro: init (wire pin1∈{1,2,3}; STATUS +0xBF4/5/6)\n");

	if (!kstrtoul(region_fn_hex, 0, &fn) && fn) {
		get_region = (get_region_t)fn;
		base = get_region(0, (u32)ipc_idx);
		if (base) {
			pr_info("saaios_sim_ro: IPC vbase=%px\n", base);
			total += scan_wire(base, 0x0000, 0x10000, "fmt");
			total += scan_wire(base, 0x00400000, 0x4000, "srinfo");
			total += scan_wire(base, 0x00200000, 0x10000, "raw");
			total += scan_status(base, 0x0000, 0x10000, "fmt");
			total += scan_status(base, 0x00400000, 0x4000, "srinfo");
		} else {
			pr_err("saaios_sim_ro: get_region NULL\n");
		}
	} else {
		pr_info("saaios_sim_ro: SHMEM skip (no region_fn)\n");
	}

	if (do_btl)
		total += scan_btl();

	pr_info("saaios_sim_ro: TOTAL=%d — poke only if unique PresentObj-class hit\n",
		total);
	return 0;
}

static void __exit saaios_sim_ro_exit(void)
{
	pr_info("saaios_sim_ro: exit\n");
}

module_init(saaios_sim_ro_init);
module_exit(saaios_sim_ro_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("SaaiOS BTL/SHMEM RO SIM-layout scan");
MODULE_AUTHOR("SaaiOS");
