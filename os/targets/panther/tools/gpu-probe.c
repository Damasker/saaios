#define _GNU_SOURCE

#include <dirent.h>
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

/*
 * Read-only graphics capability probe for Pixel 7. It never loads modules,
 * opens DRM nodes for writing, changes power state, or becomes DRM master.
 */

static bool path_exists(const char *path) {
    return access(path, F_OK) == 0;
}

static bool name_has(const char *name, const char *needle) {
    return strstr(name, needle) != NULL;
}

static int list_matching(const char *label, const char *path,
                         const char *first, const char *second) {
    DIR *directory = opendir(path);
    int count = 0;
    printf("%s:\n", label);
    if (!directory) {
        printf("  unavailable (%s)\n", strerror(errno));
        return 0;
    }
    struct dirent *entry;
    while ((entry = readdir(directory)) != NULL) {
        if (entry->d_name[0] == '.') continue;
        if ((first && name_has(entry->d_name, first)) ||
            (second && name_has(entry->d_name, second)) ||
            (!first && !second)) {
            printf("  %s\n", entry->d_name);
            count++;
        }
    }
    if (count == 0) printf("  (none)\n");
    closedir(directory);
    return count;
}

static bool module_loaded(const char *wanted) {
    FILE *file = fopen("/proc/modules", "r");
    if (!file) return false;
    char line[512];
    bool found = false;
    while (fgets(line, sizeof(line), file)) {
        char name[96];
        if (sscanf(line, "%95s", name) == 1 &&
            strcmp(name, wanted) == 0) {
            found = true;
            break;
        }
    }
    fclose(file);
    return found;
}

static bool try_egl_loader(void) {
    static const char *const names[] = {
        "libEGL.so.1", "libEGL.so",
    };
    for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); ++i) {
        void *library = dlopen(names[i], RTLD_NOW | RTLD_LOCAL);
        if (library) {
            printf("egl_loader: %s\n", names[i]);
            dlclose(library);
            return true;
        }
    }
    printf("egl_loader: unavailable\n");
    return false;
}

int main(void) {
    printf("SaaiOS panther GPU probe (read-only)\n");
    printf("expected_gpu: Mali-G710 class, Panthor path\n");
    printf("forbidden: module load, DRM master, power/sysfs writes\n\n");

    int dri_nodes = list_matching("dri_nodes", "/dev/dri", NULL, NULL);
    bool render_node = path_exists("/dev/dri/renderD128") ||
                       path_exists("/dev/dri/renderD129");
    int drm_render = list_matching("sysfs_render_nodes", "/sys/class/drm",
                                   "renderD", NULL);
    int gpu_platform = list_matching("gpu_platform_nodes",
                                     "/sys/bus/platform/devices",
                                     "mali", "gpu");
    bool panthor = module_loaded("panthor");
    bool panfrost = module_loaded("panfrost");
    bool kbase = module_loaded("mali_kbase") || module_loaded("mali");
    bool firmware = path_exists("/lib/firmware/arm/mali") ||
                    path_exists("/vendor/firmware/mali_csffw.bin") ||
                    path_exists("/lib/firmware/mali_csffw.bin");
    bool egl = try_egl_loader();

    printf("kernel_modules: panthor=%s panfrost=%s kbase=%s\n",
           panthor ? "yes" : "no",
           panfrost ? "yes" : "no",
           kbase ? "yes" : "no");
    printf("firmware_path: %s\n", firmware ? "present" : "not-found");
    printf("render_node: %s\n", render_node ? "present" : "missing");
    printf("counts: dri=%d sysfs_render=%d gpu_platform=%d\n",
           dri_nodes, drm_render, gpu_platform);

    const char *state = "NO_KERNEL_GPU";
    if (panthor || panfrost || kbase || render_node) state = "KERNEL_VISIBLE";
    if (render_node) state = "RENDER_NODE_READY";
    if (render_node && egl) state = "EGL_LOADER_READY";
    printf("summary_state=%s\n", state);
    return render_node ? 0 : 2;
}
