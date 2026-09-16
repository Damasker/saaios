/* ADR-024 continued: bounded feasibility check before attempting a full
 * async GPU submit pipeline (deferred wl_buffer.release() via a real
 * kernel-level dma-fence passed to the DRM atomic commit's IN_FENCE_FD
 * plane property). That design needs the closed Mali UMD to actually
 * support exporting a VkFence as a syncfd -- VK_KHR_external_fence_fd or
 * VK_EXT_external_fence_fd. This tool only enumerates device extensions
 * and reports whether either is present; it does not attempt to use
 * them. Same freestanding/no-libc boilerplate as vk-triangle.c/vk_probe.c
 * (this UMD is bionic-only, loaded via the same HMI dlopen path).
 */
#include <vulkan/vulkan.h>
#include <stdint.h>

extern long write(int fd, const void *buf, unsigned long count);
extern void exit(int status);
extern void *malloc(unsigned long size);

#define RTLD_NOW 2
extern void *dlopen(const char *filename, int flags);
extern void *dlsym(void *handle, const char *symbol);
extern char *dlerror(void);

static void put(const char *s)
{
	const char *p = s;
	unsigned long n = 0;
	while (p[n])
		n++;
	write(2, s, n);
}

static void put_hex(unsigned long v)
{
	char buf[17];
	const char *digits = "0123456789abcdef";
	for (int i = 15; i >= 0; i--) {
		buf[i] = digits[v & 0xf];
		v >>= 4;
	}
	buf[16] = '\0';
	put("0x");
	put(buf);
}

static int str_eq(const char *a, const char *b)
{
	while (*a && *a == *b) {
		a++;
		b++;
	}
	return *a == *b;
}

struct hw_module_t;
struct hw_device_t;

struct hw_module_methods_t {
	int (*open)(const struct hw_module_t *module, const char *id,
		    struct hw_device_t **device);
};

struct hw_module_t {
	uint32_t tag;
	uint16_t module_api_version;
	uint16_t hal_api_version;
	const char *id;
	const char *name;
	const char *author;
	struct hw_module_methods_t *methods;
	void *dso;
	uint64_t reserved[32 - 7];
};

struct hw_device_t {
	uint32_t tag;
	uint32_t version;
	struct hw_module_t *module;
	uint64_t reserved[12];
	int (*close)(struct hw_device_t *device);
};

typedef VkResult (*PFN_vkEnumerateInstanceExtensionProperties_t)(
	const char *pLayerName, uint32_t *pPropertyCount, VkExtensionProperties *pProperties);

struct hwvulkan_device_t {
	struct hw_device_t common;
	PFN_vkEnumerateInstanceExtensionProperties_t EnumerateInstanceExtensionProperties;
	PFN_vkCreateInstance CreateInstance;
	PFN_vkGetInstanceProcAddr GetInstanceProcAddr;
};

#define HWVULKAN_DEVICE_0 "vk0"

static PFN_vkGetInstanceProcAddr g_gipa;
static VkInstance g_instance;

#define LOAD(fn) PFN_##fn fn = (PFN_##fn)g_gipa(g_instance, #fn); \
	do { \
		if (!fn) { \
			put("vk-extcheck: FATAL missing " #fn "\n"); \
			exit(1); \
		} \
	} while (0)

int main(int argc, char **argv, char **envp)
{
	(void)argc;
	(void)argv;
	(void)envp;
	void *h = dlopen("/vendor/lib64/hw/vulkan.mali.so", RTLD_NOW);
	if (!h) {
		put("vk-extcheck: dlopen FAILED: ");
		put(dlerror());
		put("\n");
		return 1;
	}
	struct hw_module_t *module = (struct hw_module_t *)dlsym(h, "HMI");
	if (!module) {
		put("vk-extcheck: dlsym(HMI) FAILED\n");
		return 1;
	}
	struct hw_device_t *device_h = NULL;
	int ret = module->methods->open(module, HWVULKAN_DEVICE_0, &device_h);
	if (ret != 0 || !device_h) {
		put("vk-extcheck: open() failed\n");
		return 1;
	}
	struct hwvulkan_device_t *vkdev = (struct hwvulkan_device_t *)device_h;
	if (!vkdev->CreateInstance || !vkdev->GetInstanceProcAddr) {
		put("vk-extcheck: NULL CreateInstance/GetInstanceProcAddr\n");
		return 1;
	}
	g_gipa = vkdev->GetInstanceProcAddr;

	VkApplicationInfo app_info = {
		.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
		.pApplicationName = "vk-extcheck",
		.apiVersion = VK_API_VERSION_1_0,
	};
	VkInstanceCreateInfo inst_info = {
		.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
		.pApplicationInfo = &app_info,
	};
	VkResult r = vkdev->CreateInstance(&inst_info, NULL, &g_instance);
	if (r != VK_SUCCESS) {
		put("vk-extcheck: CreateInstance failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	LOAD(vkEnumeratePhysicalDevices);
	LOAD(vkEnumerateDeviceExtensionProperties);

	uint32_t pd_count = 1;
	VkPhysicalDevice phys;
	r = vkEnumeratePhysicalDevices(g_instance, &pd_count, &phys);
	if (r != VK_SUCCESS || pd_count == 0) {
		put("vk-extcheck: no physical device\n");
		return 1;
	}

	uint32_t ext_count = 0;
	r = vkEnumerateDeviceExtensionProperties(phys, NULL, &ext_count, NULL);
	if (r != VK_SUCCESS) {
		put("vk-extcheck: EnumerateDeviceExtensionProperties (count) failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	put("vk-extcheck: device extension count = ");
	{
		char buf[16];
		int i = 15;
		buf[i--] = '\0';
		unsigned long v = ext_count;
		if (v == 0) buf[i--] = '0';
		while (v > 0) { buf[i--] = '0' + (v % 10); v /= 10; }
		put(&buf[i + 1]);
	}
	put("\n");

	VkExtensionProperties *props = (VkExtensionProperties *)malloc(
		(unsigned long)ext_count * sizeof(VkExtensionProperties));
	if (!props) {
		put("vk-extcheck: malloc failed\n");
		return 1;
	}
	r = vkEnumerateDeviceExtensionProperties(phys, NULL, &ext_count, props);
	if (r != VK_SUCCESS) {
		put("vk-extcheck: EnumerateDeviceExtensionProperties (list) failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	int has_khr_fence_fd = 0;
	int has_ext_fence_fd = 0;
	int has_khr_semaphore_fd = 0;
	for (uint32_t i = 0; i < ext_count; i++) {
		put("  ");
		put(props[i].extensionName);
		put("\n");
		if (str_eq(props[i].extensionName, "VK_KHR_external_fence_fd"))
			has_khr_fence_fd = 1;
		if (str_eq(props[i].extensionName, "VK_EXT_external_fence_fd"))
			has_ext_fence_fd = 1;
		if (str_eq(props[i].extensionName, "VK_KHR_external_semaphore_fd"))
			has_khr_semaphore_fd = 1;
	}

	put("vk-extcheck: VK_KHR_external_fence_fd = ");
	put(has_khr_fence_fd ? "YES" : "no");
	put("\n");
	put("vk-extcheck: VK_EXT_external_fence_fd = ");
	put(has_ext_fence_fd ? "YES" : "no");
	put("\n");
	put("vk-extcheck: VK_KHR_external_semaphore_fd = ");
	put(has_khr_semaphore_fd ? "YES" : "no");
	put("\n");

	return 0;
}
