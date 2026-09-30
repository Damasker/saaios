/* RO scan SHMEM_IPC via existing cp_shmem_get_region (no second ioremap).
 * Pass region_fn = kallsyms cp_shmem_get_region. SHMEM_IPC idx=3 on panther.
 * No writes.
 */
#include <linux/module.h>
#include <linux/io.h>
#include <linux/kstrtox.h>

typedef void __iomem *(*get_region_t)(u32 cp, u32 idx);

static char region_fn_hex[24] = "0";
module_param_string(region_fn, region_fn_hex, sizeof(region_fn_hex), 0644);

static int ipc_idx = 3; /* DT regions/ipc region,index */
module_param(ipc_idx, int, 0644);

static int scan_pat(void __iomem *base, u32 off0, u32 len, const char *tag)
{
	u32 off;
	int hits = 0;
	int i;
	char hex[3 * 48];
	int hi = 0;

	for (i = 0; i < 32 && (u32)i < len; i++)
		hi += scnprintf(hex + hi, sizeof(hex) - hi, "%02x%s",
				readb(base + off0 + i), i + 1 < 32 ? " " : "");
	pr_info("saaios_shm_ro: %s +0x%x HDR32 %s\n", tag, off0, hex);

	for (off = 0; off + 64 < len; off++) {
		u8 card = readb(base + off0 + off);
		u8 apps = readb(base + off0 + off + 2);
		u8 type = readb(base + off0 + off + 3);
		u8 state = readb(base + off0 + off + 5);
		u8 pin1 = readb(base + off0 + off + 60);
		if (card != 1 || apps != 1 || type != 2)
			continue;
		if (state != 2 && state != 1 && state != 5)
			continue;
		if (pin1 != 3 && pin1 != 1 && pin1 != 0 && pin1 != 2)
			continue;
		pr_info("saaios_shm_ro: SIM-PAT %s off=0x%x state=%u pin1=%u\n",
			tag, off0 + off, state, pin1);
		hits++;
		if (hits >= 8)
			break;
	}
	pr_info("saaios_shm_ro: %s hits=%d\n", tag, hits);
	return hits;
}

static int __init saaios_shm_ro_init(void)
{
	get_region_t get_region;
	unsigned long fn;
	void __iomem *base;
	int total = 0;

	if (kstrtoul(region_fn_hex, 0, &fn) || !fn) {
		pr_err("saaios_shm_ro: need region_fn=cp_shmem_get_region\n");
		return -EINVAL;
	}
	get_region = (get_region_t)fn;
	base = get_region(0, (u32)ipc_idx);
	if (!base) {
		pr_err("saaios_shm_ro: get_region(0,%d) NULL\n", ipc_idx);
		return -ENODEV;
	}
	pr_info("saaios_shm_ro: IPC vbase=%px idx=%d (RO)\n", base, ipc_idx);

	pr_info("saaios_shm_ro: magic=0x%08x mem_access=0x%08x united@0x4000=0x%08x\n",
		ioread32(base + 0x0), ioread32(base + 0x4), ioread32(base + 0x4000));
	pr_info("saaios_shm_ro: srinfo[0]=0x%08x\n", ioread32(base + 0x00400000));

	total += scan_pat(base, 0x0000, 0x8000, "fmt+ctrl");
	total += scan_pat(base, 0x00400000, 0x4000, "srinfo");
	total += scan_pat(base, 0x00200000, 0x8000, "raw_rx_head");

	pr_info("saaios_shm_ro: TOTAL_HITS=%d — ring copies only; not PresentObj/+0xBF6; no poke\n",
		total);
	/* do NOT vunmap — owned by shm_ipc */
	return 0;
}

static void __exit saaios_shm_ro_exit(void)
{
	pr_info("saaios_shm_ro: exit\n");
}

module_init(saaios_shm_ro_init);
module_exit(saaios_shm_ro_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("SaaiOS SHMEM_IPC RO SIM-pattern scan");
MODULE_AUTHOR("SaaiOS");
