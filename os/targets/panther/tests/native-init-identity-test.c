#define main panther_native_init_main
#include "../src/native-init.c"
#undef main

#include <assert.h>

static int fake_partition(const char *name, const char *node) {
    (void)name;
    (void)node;
    return 0;
}

static int fail_mount(const char *source, const char *target,
                      const char *filesystemtype, unsigned long mountflags,
                      const void *data) {
    (void)source;
    (void)target;
    (void)filesystemtype;
    (void)mountflags;
    (void)data;
    errno = ENODEV;
    return -1;
}

int main(void) {
    unsetenv("SAAIOS_DEPLOYMENT");
    unsetenv("SAAIOS_DEVICE_CLASS");
    unsetenv("SAAIOS_DEVICE_TARGET");
    setenv("SAAIOS_DATA", "/stale/unmounted/path", 1);

    setup_platform_identity();
    setup_data_storage_with(fake_partition, fail_mount);

    assert(strcmp(getenv("SAAIOS_DEPLOYMENT"), "native_device") == 0);
    assert(strcmp(getenv("SAAIOS_DEVICE_CLASS"), "phone") == 0);
    assert(strcmp(getenv("SAAIOS_DEVICE_TARGET"), "panther") == 0);
    assert(getenv("SAAIOS_DATA") == NULL);
    return 0;
}
