/* SaaiOS /sbin/poweroff — Phase D probe (bringup-plan.md / hardware-control-plan.md).
 * /sys/power/state is freeze-mem only (suspend states); that says
 * nothing about reboot(2) RB_POWER_OFF, a separate kernel path that
 * calls kernel_power_off() -> the board's pm_power_off hook. Stock
 * Samsung kernels normally wire that hook for a real hardware
 * shutdown. Telnet-only probe: confirm this actually powers the
 * device off (not a hang, not a silent return) before wiring any
 * key gesture to it. Do NOT rebind the existing 2s long-press ->
 * reboot; kernel-touch.md's recovery flow depends on that exact
 * behavior when a touch experiment goes state=dead.
 */
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/reboot.h>
#include <unistd.h>

int main(void) {
	sync();
	fprintf(stderr, "poweroff: calling reboot(RB_POWER_OFF)\n");
	reboot(RB_POWER_OFF);
	fprintf(stderr, "poweroff: reboot() returned instead of powering off: %s\n",
		strerror(errno));
	return 1;
}
