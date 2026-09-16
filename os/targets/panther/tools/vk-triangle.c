/* ADR-024 continued: real 3D rasterization check, not just a render-pass
 * clear (vk-frame.c only ever does vkCmdBeginRenderPass/EndRenderPass
 * with loadOp=CLEAR -- no vertex/fragment shader pipeline, no vkCmdDraw,
 * so it never actually exercises the rasterizer). This variant creates a
 * real VkPipeline (hardcoded-position vertex shader, per-vertex color
 * interpolated by a fragment shader across three vertices via
 * gl_VertexIndex -- no vertex buffer needed) and issues a real
 * vkCmdDraw(3,1,0,0) triangle inside the render pass, then verifies on
 * the CPU side that a background corner pixel stayed the clear color
 * while a pixel inside the triangle shows a real interpolated blend --
 * proof the rasterizer/vertex-shader/fragment-shader pipeline actually
 * ran, not just that a solid clear color was written.
 */
#include <vulkan/vulkan.h>
#include <stdint.h>

extern long write(int fd, const void *buf, unsigned long count);
extern void exit(int status);
extern void *memset(void *s, int c, unsigned long n);

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

static void put_dec(unsigned long v)
{
	char buf[24];
	int i = 23;
	buf[i--] = '\0';
	if (v == 0)
		buf[i--] = '0';
	while (v > 0) {
		buf[i--] = '0' + (v % 10);
		v /= 10;
	}
	put(&buf[i + 1]);
}

/* --- Real, stable AOSP ABI structs, see vk_probe.c for provenance. --- */

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
#define IMG_W 64
#define IMG_H 64

/* --- Embedded SPIR-V: triangle.vert / triangle.frag (glslc, GLSL 450). ---
 * vert: hardcoded NDC positions + per-vertex RGB color, indexed by
 * gl_VertexIndex, no vertex buffer.
 * frag: outputs the rasterizer-interpolated vColor.
 */
static const uint32_t g_vert_spv[] = {
	0x07230203u, 0x00010000u, 0x000d000au, 0x00000036u, 0x00000000u, 0x00020011u, 0x00000001u, 0x0006000bu,
	0x00000001u, 0x4c534c47u, 0x6474732eu, 0x3035342eu, 0x00000000u, 0x0003000eu, 0x00000000u, 0x00000001u,
	0x0008000fu, 0x00000000u, 0x00000004u, 0x6e69616du, 0x00000000u, 0x00000022u, 0x00000026u, 0x00000031u,
	0x00030003u, 0x00000002u, 0x000001c2u, 0x000a0004u, 0x475f4c47u, 0x4c474f4fu, 0x70635f45u, 0x74735f70u,
	0x5f656c79u, 0x656e696cu, 0x7269645fu, 0x69746365u, 0x00006576u, 0x00080004u, 0x475f4c47u, 0x4c474f4fu,
	0x6e695f45u, 0x64756c63u, 0x69645f65u, 0x74636572u, 0x00657669u, 0x00040005u, 0x00000004u, 0x6e69616du,
	0x00000000u, 0x00050005u, 0x0000000cu, 0x69736f70u, 0x6e6f6974u, 0x00000073u, 0x00040005u, 0x00000017u,
	0x6f6c6f63u, 0x00007372u, 0x00060005u, 0x00000020u, 0x505f6c67u, 0x65567265u, 0x78657472u, 0x00000000u,
	0x00060006u, 0x00000020u, 0x00000000u, 0x505f6c67u, 0x7469736fu, 0x006e6f69u, 0x00070006u, 0x00000020u,
	0x00000001u, 0x505f6c67u, 0x746e696fu, 0x657a6953u, 0x00000000u, 0x00070006u, 0x00000020u, 0x00000002u,
	0x435f6c67u, 0x4470696cu, 0x61747369u, 0x0065636eu, 0x00070006u, 0x00000020u, 0x00000003u, 0x435f6c67u,
	0x446c6c75u, 0x61747369u, 0x0065636eu, 0x00030005u, 0x00000022u, 0x00000000u, 0x00060005u, 0x00000026u,
	0x565f6c67u, 0x65747265u, 0x646e4978u, 0x00007865u, 0x00040005u, 0x00000031u, 0x6c6f4376u, 0x0000726fu,
	0x00050048u, 0x00000020u, 0x00000000u, 0x0000000bu, 0x00000000u, 0x00050048u, 0x00000020u, 0x00000001u,
	0x0000000bu, 0x00000001u, 0x00050048u, 0x00000020u, 0x00000002u, 0x0000000bu, 0x00000003u, 0x00050048u,
	0x00000020u, 0x00000003u, 0x0000000bu, 0x00000004u, 0x00030047u, 0x00000020u, 0x00000002u, 0x00040047u,
	0x00000026u, 0x0000000bu, 0x0000002au, 0x00040047u, 0x00000031u, 0x0000001eu, 0x00000000u, 0x00020013u,
	0x00000002u, 0x00030021u, 0x00000003u, 0x00000002u, 0x00030016u, 0x00000006u, 0x00000020u, 0x00040017u,
	0x00000007u, 0x00000006u, 0x00000002u, 0x00040015u, 0x00000008u, 0x00000020u, 0x00000000u, 0x0004002bu,
	0x00000008u, 0x00000009u, 0x00000003u, 0x0004001cu, 0x0000000au, 0x00000007u, 0x00000009u, 0x00040020u,
	0x0000000bu, 0x00000006u, 0x0000000au, 0x0004003bu, 0x0000000bu, 0x0000000cu, 0x00000006u, 0x0004002bu,
	0x00000006u, 0x0000000du, 0xbf4ccccdu, 0x0005002cu, 0x00000007u, 0x0000000eu, 0x0000000du, 0x0000000du,
	0x0004002bu, 0x00000006u, 0x0000000fu, 0x3f4ccccdu, 0x0005002cu, 0x00000007u, 0x00000010u, 0x0000000fu,
	0x0000000du, 0x0004002bu, 0x00000006u, 0x00000011u, 0x00000000u, 0x0005002cu, 0x00000007u, 0x00000012u,
	0x00000011u, 0x0000000fu, 0x0006002cu, 0x0000000au, 0x00000013u, 0x0000000eu, 0x00000010u, 0x00000012u,
	0x00040017u, 0x00000014u, 0x00000006u, 0x00000003u, 0x0004001cu, 0x00000015u, 0x00000014u, 0x00000009u,
	0x00040020u, 0x00000016u, 0x00000006u, 0x00000015u, 0x0004003bu, 0x00000016u, 0x00000017u, 0x00000006u,
	0x0004002bu, 0x00000006u, 0x00000018u, 0x3f800000u, 0x0006002cu, 0x00000014u, 0x00000019u, 0x00000018u,
	0x00000011u, 0x00000011u, 0x0006002cu, 0x00000014u, 0x0000001au, 0x00000011u, 0x00000018u, 0x00000011u,
	0x0006002cu, 0x00000014u, 0x0000001bu, 0x00000011u, 0x00000011u, 0x00000018u, 0x0006002cu, 0x00000015u,
	0x0000001cu, 0x00000019u, 0x0000001au, 0x0000001bu, 0x00040017u, 0x0000001du, 0x00000006u, 0x00000004u,
	0x0004002bu, 0x00000008u, 0x0000001eu, 0x00000001u, 0x0004001cu, 0x0000001fu, 0x00000006u, 0x0000001eu,
	0x0006001eu, 0x00000020u, 0x0000001du, 0x00000006u, 0x0000001fu, 0x0000001fu, 0x00040020u, 0x00000021u,
	0x00000003u, 0x00000020u, 0x0004003bu, 0x00000021u, 0x00000022u, 0x00000003u, 0x00040015u, 0x00000023u,
	0x00000020u, 0x00000001u, 0x0004002bu, 0x00000023u, 0x00000024u, 0x00000000u, 0x00040020u, 0x00000025u,
	0x00000001u, 0x00000023u, 0x0004003bu, 0x00000025u, 0x00000026u, 0x00000001u, 0x00040020u, 0x00000028u,
	0x00000006u, 0x00000007u, 0x00040020u, 0x0000002eu, 0x00000003u, 0x0000001du, 0x00040020u, 0x00000030u,
	0x00000003u, 0x00000014u, 0x0004003bu, 0x00000030u, 0x00000031u, 0x00000003u, 0x00040020u, 0x00000033u,
	0x00000006u, 0x00000014u, 0x00050036u, 0x00000002u, 0x00000004u, 0x00000000u, 0x00000003u, 0x000200f8u,
	0x00000005u, 0x0003003eu, 0x0000000cu, 0x00000013u, 0x0003003eu, 0x00000017u, 0x0000001cu, 0x0004003du,
	0x00000023u, 0x00000027u, 0x00000026u, 0x00050041u, 0x00000028u, 0x00000029u, 0x0000000cu, 0x00000027u,
	0x0004003du, 0x00000007u, 0x0000002au, 0x00000029u, 0x00050051u, 0x00000006u, 0x0000002bu, 0x0000002au,
	0x00000000u, 0x00050051u, 0x00000006u, 0x0000002cu, 0x0000002au, 0x00000001u, 0x00070050u, 0x0000001du,
	0x0000002du, 0x0000002bu, 0x0000002cu, 0x00000011u, 0x00000018u, 0x00050041u, 0x0000002eu, 0x0000002fu,
	0x00000022u, 0x00000024u, 0x0003003eu, 0x0000002fu, 0x0000002du, 0x0004003du, 0x00000023u, 0x00000032u,
	0x00000026u, 0x00050041u, 0x00000033u, 0x00000034u, 0x00000017u, 0x00000032u, 0x0004003du, 0x00000014u,
	0x00000035u, 0x00000034u, 0x0003003eu, 0x00000031u, 0x00000035u, 0x000100fdu, 0x00010038u,
};
static const unsigned long g_vert_spv_size = 1500;

static const uint32_t g_frag_spv[] = {
	0x07230203u, 0x00010000u, 0x000d000au, 0x00000013u, 0x00000000u, 0x00020011u, 0x00000001u, 0x0006000bu,
	0x00000001u, 0x4c534c47u, 0x6474732eu, 0x3035342eu, 0x00000000u, 0x0003000eu, 0x00000000u, 0x00000001u,
	0x0007000fu, 0x00000004u, 0x00000004u, 0x6e69616du, 0x00000000u, 0x00000009u, 0x0000000cu, 0x00030010u,
	0x00000004u, 0x00000007u, 0x00030003u, 0x00000002u, 0x000001c2u, 0x000a0004u, 0x475f4c47u, 0x4c474f4fu,
	0x70635f45u, 0x74735f70u, 0x5f656c79u, 0x656e696cu, 0x7269645fu, 0x69746365u, 0x00006576u, 0x00080004u,
	0x475f4c47u, 0x4c474f4fu, 0x6e695f45u, 0x64756c63u, 0x69645f65u, 0x74636572u, 0x00657669u, 0x00040005u,
	0x00000004u, 0x6e69616du, 0x00000000u, 0x00050005u, 0x00000009u, 0x4374756fu, 0x726f6c6fu, 0x00000000u,
	0x00040005u, 0x0000000cu, 0x6c6f4376u, 0x0000726fu, 0x00040047u, 0x00000009u, 0x0000001eu, 0x00000000u,
	0x00040047u, 0x0000000cu, 0x0000001eu, 0x00000000u, 0x00020013u, 0x00000002u, 0x00030021u, 0x00000003u,
	0x00000002u, 0x00030016u, 0x00000006u, 0x00000020u, 0x00040017u, 0x00000007u, 0x00000006u, 0x00000004u,
	0x00040020u, 0x00000008u, 0x00000003u, 0x00000007u, 0x0004003bu, 0x00000008u, 0x00000009u, 0x00000003u,
	0x00040017u, 0x0000000au, 0x00000006u, 0x00000003u, 0x00040020u, 0x0000000bu, 0x00000001u, 0x0000000au,
	0x0004003bu, 0x0000000bu, 0x0000000cu, 0x00000001u, 0x0004002bu, 0x00000006u, 0x0000000eu, 0x3f800000u,
	0x00050036u, 0x00000002u, 0x00000004u, 0x00000000u, 0x00000003u, 0x000200f8u, 0x00000005u, 0x0004003du,
	0x0000000au, 0x0000000du, 0x0000000cu, 0x00050051u, 0x00000006u, 0x0000000fu, 0x0000000du, 0x00000000u,
	0x00050051u, 0x00000006u, 0x00000010u, 0x0000000du, 0x00000001u, 0x00050051u, 0x00000006u, 0x00000011u,
	0x0000000du, 0x00000002u, 0x00070050u, 0x00000007u, 0x00000012u, 0x0000000fu, 0x00000010u, 0x00000011u,
	0x0000000eu, 0x0003003eu, 0x00000009u, 0x00000012u, 0x000100fdu, 0x00010038u,
};
static const unsigned long g_frag_spv_size = 568;

static PFN_vkGetInstanceProcAddr g_gipa;
static VkInstance g_instance;

#define LOAD(fn) PFN_##fn fn = (PFN_##fn)g_gipa(g_instance, #fn); \
	do { \
		if (!fn) { \
			put("vk-triangle: FATAL missing " #fn "\n"); \
			exit(1); \
		} \
	} while (0)

static uint32_t find_memory_type(const VkPhysicalDeviceMemoryProperties *mp,
				  uint32_t type_bits, VkMemoryPropertyFlags want)
{
	for (uint32_t i = 0; i < mp->memoryTypeCount; i++) {
		if ((type_bits & (1u << i)) &&
		    (mp->memoryTypes[i].propertyFlags & want) == want)
			return i;
	}
	return 0xffffffff;
}

int main(int argc, char **argv, char **envp)
{
	(void)argc;
	(void)argv;
	(void)envp;
	put("vk-triangle: dlopen(/vendor/lib64/hw/vulkan.mali.so)\n");
	void *h = dlopen("/vendor/lib64/hw/vulkan.mali.so", RTLD_NOW);
	if (!h) {
		put("vk-triangle: dlopen FAILED: ");
		put(dlerror());
		put("\n");
		return 1;
	}

	struct hw_module_t *module = (struct hw_module_t *)dlsym(h, "HMI");
	if (!module) {
		put("vk-triangle: dlsym(HMI) FAILED\n");
		return 1;
	}

	struct hw_device_t *device_h = NULL;
	int ret = module->methods->open(module, HWVULKAN_DEVICE_0, &device_h);
	if (ret != 0 || !device_h) {
		put("vk-triangle: open() failed\n");
		return 1;
	}

	struct hwvulkan_device_t *vkdev = (struct hwvulkan_device_t *)device_h;
	if (!vkdev->CreateInstance || !vkdev->GetInstanceProcAddr) {
		put("vk-triangle: NULL CreateInstance/GetInstanceProcAddr\n");
		return 1;
	}
	g_gipa = vkdev->GetInstanceProcAddr;

	VkApplicationInfo app_info = {
		.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
		.pApplicationName = "vk-triangle",
		.apiVersion = VK_API_VERSION_1_0,
	};
	VkInstanceCreateInfo inst_info = {
		.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
		.pApplicationInfo = &app_info,
	};
	VkResult r = vkdev->CreateInstance(&inst_info, NULL, &g_instance);
	if (r != VK_SUCCESS) {
		put("vk-triangle: CreateInstance failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	put("vk-triangle: VkInstance created\n");

	LOAD(vkEnumeratePhysicalDevices);
	LOAD(vkGetPhysicalDeviceQueueFamilyProperties);
	LOAD(vkGetPhysicalDeviceMemoryProperties);
	LOAD(vkCreateDevice);
	LOAD(vkGetDeviceQueue);
	LOAD(vkCreateImage);
	LOAD(vkGetImageMemoryRequirements);
	LOAD(vkAllocateMemory);
	LOAD(vkBindImageMemory);
	LOAD(vkCreateImageView);
	LOAD(vkCreateRenderPass);
	LOAD(vkCreateFramebuffer);
	LOAD(vkCreateShaderModule);
	LOAD(vkCreatePipelineLayout);
	LOAD(vkCreateGraphicsPipelines);
	LOAD(vkCreateCommandPool);
	LOAD(vkAllocateCommandBuffers);
	LOAD(vkBeginCommandBuffer);
	LOAD(vkCmdBeginRenderPass);
	LOAD(vkCmdBindPipeline);
	LOAD(vkCmdDraw);
	LOAD(vkCmdEndRenderPass);
	LOAD(vkCmdCopyImageToBuffer);
	LOAD(vkCmdPipelineBarrier);
	LOAD(vkEndCommandBuffer);
	LOAD(vkCreateBuffer);
	LOAD(vkGetBufferMemoryRequirements);
	LOAD(vkBindBufferMemory);
	LOAD(vkQueueSubmit);
	LOAD(vkQueueWaitIdle);
	LOAD(vkMapMemory);
	LOAD(vkUnmapMemory);
	put("vk-triangle: all function pointers loaded\n");

	uint32_t pd_count = 1;
	VkPhysicalDevice phys;
	r = vkEnumeratePhysicalDevices(g_instance, &pd_count, &phys);
	if (r != VK_SUCCESS || pd_count == 0) {
		put("vk-triangle: no physical device\n");
		return 1;
	}

	VkPhysicalDeviceMemoryProperties mem_props;
	vkGetPhysicalDeviceMemoryProperties(phys, &mem_props);

	uint32_t qf_count = 8;
	VkQueueFamilyProperties qfp[8];
	vkGetPhysicalDeviceQueueFamilyProperties(phys, &qf_count, qfp);
	uint32_t qfi = 0xffffffff;
	for (uint32_t i = 0; i < qf_count; i++) {
		if (qfp[i].queueFlags & VK_QUEUE_GRAPHICS_BIT) {
			qfi = i;
			break;
		}
	}
	if (qfi == 0xffffffff) {
		put("vk-triangle: no graphics queue family\n");
		return 1;
	}
	put("vk-triangle: using queue family ");
	put_dec(qfi);
	put("\n");

	float priority = 1.0f;
	VkDeviceQueueCreateInfo qci = {
		.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
		.queueFamilyIndex = qfi,
		.queueCount = 1,
		.pQueuePriorities = &priority,
	};
	VkDeviceCreateInfo dci = {
		.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
		.queueCreateInfoCount = 1,
		.pQueueCreateInfos = &qci,
	};
	VkDevice dev;
	r = vkCreateDevice(phys, &dci, NULL, &dev);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreateDevice failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	put("vk-triangle: VkDevice created\n");

	VkQueue queue;
	vkGetDeviceQueue(dev, qfi, 0, &queue);

	/* --- Color image (render target) --- */
	VkFormat format = VK_FORMAT_R8G8B8A8_UNORM;
	VkImageCreateInfo image_ci = {
		.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
		.imageType = VK_IMAGE_TYPE_2D,
		.format = format,
		.extent = { IMG_W, IMG_H, 1 },
		.mipLevels = 1,
		.arrayLayers = 1,
		.samples = VK_SAMPLE_COUNT_1_BIT,
		.tiling = VK_IMAGE_TILING_OPTIMAL,
		.usage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_TRANSFER_SRC_BIT,
		.sharingMode = VK_SHARING_MODE_EXCLUSIVE,
		.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED,
	};
	VkImage image;
	r = vkCreateImage(dev, &image_ci, NULL, &image);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreateImage failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	VkMemoryRequirements img_mr;
	vkGetImageMemoryRequirements(dev, image, &img_mr);
	uint32_t img_mem_type = find_memory_type(&mem_props, img_mr.memoryTypeBits,
						  VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);
	if (img_mem_type == 0xffffffff)
		img_mem_type = find_memory_type(&mem_props, img_mr.memoryTypeBits, 0);
	VkMemoryAllocateInfo img_mai = {
		.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
		.allocationSize = img_mr.size,
		.memoryTypeIndex = img_mem_type,
	};
	VkDeviceMemory image_mem;
	r = vkAllocateMemory(dev, &img_mai, NULL, &image_mem);
	if (r != VK_SUCCESS) {
		put("vk-triangle: image vkAllocateMemory failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	vkBindImageMemory(dev, image, image_mem, 0);
	put("vk-triangle: color image created and bound\n");

	VkImageViewCreateInfo view_ci = {
		.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
		.image = image,
		.viewType = VK_IMAGE_VIEW_TYPE_2D,
		.format = format,
		.subresourceRange = {
			.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT,
			.levelCount = 1,
			.layerCount = 1,
		},
	};
	VkImageView view;
	r = vkCreateImageView(dev, &view_ci, NULL, &view);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreateImageView failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	/* --- Render pass: single color attachment, clear to black, then
	 * transition to TRANSFER_SRC for the copy-out below. --- */
	VkAttachmentDescription attach = {
		.format = format,
		.samples = VK_SAMPLE_COUNT_1_BIT,
		.loadOp = VK_ATTACHMENT_LOAD_OP_CLEAR,
		.storeOp = VK_ATTACHMENT_STORE_OP_STORE,
		.stencilLoadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE,
		.stencilStoreOp = VK_ATTACHMENT_STORE_OP_DONT_CARE,
		.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED,
		.finalLayout = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
	};
	VkAttachmentReference attach_ref = {
		.attachment = 0,
		.layout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
	};
	VkSubpassDescription subpass = {
		.pipelineBindPoint = VK_PIPELINE_BIND_POINT_GRAPHICS,
		.colorAttachmentCount = 1,
		.pColorAttachments = &attach_ref,
	};
	VkSubpassDependency render_to_copy = {
		.srcSubpass = 0,
		.dstSubpass = VK_SUBPASS_EXTERNAL,
		.srcStageMask = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
		.dstStageMask = VK_PIPELINE_STAGE_TRANSFER_BIT,
		.srcAccessMask = VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
		.dstAccessMask = VK_ACCESS_TRANSFER_READ_BIT,
		.dependencyFlags = VK_DEPENDENCY_BY_REGION_BIT,
	};
	VkRenderPassCreateInfo rp_ci = {
		.sType = VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
		.attachmentCount = 1,
		.pAttachments = &attach,
		.subpassCount = 1,
		.pSubpasses = &subpass,
		.dependencyCount = 1,
		.pDependencies = &render_to_copy,
	};
	VkRenderPass render_pass;
	r = vkCreateRenderPass(dev, &rp_ci, NULL, &render_pass);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreateRenderPass failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	VkFramebufferCreateInfo fb_ci = {
		.sType = VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
		.renderPass = render_pass,
		.attachmentCount = 1,
		.pAttachments = &view,
		.width = IMG_W,
		.height = IMG_H,
		.layers = 1,
	};
	VkFramebuffer framebuffer;
	r = vkCreateFramebuffer(dev, &fb_ci, NULL, &framebuffer);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreateFramebuffer failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	put("vk-triangle: render pass + framebuffer created\n");

	/* --- Real graphics pipeline: vertex + fragment shader modules,
	 * empty pipeline layout (no descriptors/push constants), fixed
	 * IMG_W x IMG_H viewport/scissor (no dynamic state), one
	 * triangle-list draw with no vertex buffers -- positions and
	 * per-vertex color come from gl_VertexIndex inside the shader. --- */
	VkShaderModuleCreateInfo vert_ci = {
		.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
		.codeSize = g_vert_spv_size,
		.pCode = g_vert_spv,
	};
	VkShaderModule vert_module;
	r = vkCreateShaderModule(dev, &vert_ci, NULL, &vert_module);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreateShaderModule(vert) failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	VkShaderModuleCreateInfo frag_ci = {
		.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
		.codeSize = g_frag_spv_size,
		.pCode = g_frag_spv,
	};
	VkShaderModule frag_module;
	r = vkCreateShaderModule(dev, &frag_ci, NULL, &frag_module);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreateShaderModule(frag) failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	put("vk-triangle: shader modules created\n");

	VkPipelineShaderStageCreateInfo stages[2] = {
		{
			.sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
			.stage = VK_SHADER_STAGE_VERTEX_BIT,
			.module = vert_module,
			.pName = "main",
		},
		{
			.sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
			.stage = VK_SHADER_STAGE_FRAGMENT_BIT,
			.module = frag_module,
			.pName = "main",
		},
	};

	VkPipelineVertexInputStateCreateInfo vertex_input = {
		.sType = VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
	};
	VkPipelineInputAssemblyStateCreateInfo input_assembly = {
		.sType = VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
		.topology = VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
	};
	VkViewport viewport = { 0.0f, 0.0f, (float)IMG_W, (float)IMG_H, 0.0f, 1.0f };
	VkRect2D scissor = { { 0, 0 }, { IMG_W, IMG_H } };
	VkPipelineViewportStateCreateInfo viewport_state = {
		.sType = VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
		.viewportCount = 1,
		.pViewports = &viewport,
		.scissorCount = 1,
		.pScissors = &scissor,
	};
	VkPipelineRasterizationStateCreateInfo raster = {
		.sType = VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
		.polygonMode = VK_POLYGON_MODE_FILL,
		.cullMode = VK_CULL_MODE_NONE,
		.frontFace = VK_FRONT_FACE_COUNTER_CLOCKWISE,
		.lineWidth = 1.0f,
	};
	VkPipelineMultisampleStateCreateInfo multisample = {
		.sType = VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
		.rasterizationSamples = VK_SAMPLE_COUNT_1_BIT,
	};
	VkPipelineColorBlendAttachmentState blend_attach = {
		.colorWriteMask = VK_COLOR_COMPONENT_R_BIT | VK_COLOR_COMPONENT_G_BIT |
				   VK_COLOR_COMPONENT_B_BIT | VK_COLOR_COMPONENT_A_BIT,
	};
	VkPipelineColorBlendStateCreateInfo blend_state = {
		.sType = VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
		.attachmentCount = 1,
		.pAttachments = &blend_attach,
	};

	VkPipelineLayoutCreateInfo layout_ci = {
		.sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
	};
	VkPipelineLayout pipeline_layout;
	r = vkCreatePipelineLayout(dev, &layout_ci, NULL, &pipeline_layout);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreatePipelineLayout failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	VkGraphicsPipelineCreateInfo pipeline_ci = {
		.sType = VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
		.stageCount = 2,
		.pStages = stages,
		.pVertexInputState = &vertex_input,
		.pInputAssemblyState = &input_assembly,
		.pViewportState = &viewport_state,
		.pRasterizationState = &raster,
		.pMultisampleState = &multisample,
		.pColorBlendState = &blend_state,
		.layout = pipeline_layout,
		.renderPass = render_pass,
		.subpass = 0,
	};
	VkPipeline pipeline;
	r = vkCreateGraphicsPipelines(dev, VK_NULL_HANDLE, 1, &pipeline_ci, NULL, &pipeline);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreateGraphicsPipelines failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	put("vk-triangle: graphics pipeline created\n");

	/* --- Readback buffer (host-visible) --- */
	VkDeviceSize buf_size = (VkDeviceSize)IMG_W * IMG_H * 4;
	VkBufferCreateInfo buf_ci = {
		.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
		.size = buf_size,
		.usage = VK_BUFFER_USAGE_TRANSFER_DST_BIT,
		.sharingMode = VK_SHARING_MODE_EXCLUSIVE,
	};
	VkBuffer readback_buf;
	r = vkCreateBuffer(dev, &buf_ci, NULL, &readback_buf);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreateBuffer failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	VkMemoryRequirements buf_mr;
	vkGetBufferMemoryRequirements(dev, readback_buf, &buf_mr);
	uint32_t buf_mem_type = find_memory_type(&mem_props, buf_mr.memoryTypeBits,
						  VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT |
						  VK_MEMORY_PROPERTY_HOST_COHERENT_BIT);
	if (buf_mem_type == 0xffffffff) {
		put("vk-triangle: no HOST_VISIBLE|HOST_COHERENT memory type\n");
		return 1;
	}
	VkMemoryAllocateInfo buf_mai = {
		.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
		.allocationSize = buf_mr.size,
		.memoryTypeIndex = buf_mem_type,
	};
	VkDeviceMemory buf_mem;
	r = vkAllocateMemory(dev, &buf_mai, NULL, &buf_mem);
	if (r != VK_SUCCESS) {
		put("vk-triangle: buffer vkAllocateMemory failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	vkBindBufferMemory(dev, readback_buf, buf_mem, 0);
	/* Poison the destination before submission so an omitted copy cannot be
	 * mistaken for a legitimately rendered frame. */
	void *initial_map;
	r = vkMapMemory(dev, buf_mem, 0, buf_size, 0, &initial_map);
	if (r != VK_SUCCESS) {
		put("vk-triangle: initial vkMapMemory failed\n");
		return 1;
	}
	memset(initial_map, 0x11, (unsigned long)buf_size);
	vkUnmapMemory(dev, buf_mem);
	put("vk-triangle: readback buffer created and bound\n");

	/* --- Command pool + buffer --- */
	VkCommandPoolCreateInfo pool_ci = {
		.sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
		.queueFamilyIndex = qfi,
	};
	VkCommandPool cmd_pool;
	r = vkCreateCommandPool(dev, &pool_ci, NULL, &cmd_pool);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkCreateCommandPool failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	VkCommandBufferAllocateInfo cb_ai = {
		.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
		.commandPool = cmd_pool,
		.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY,
		.commandBufferCount = 1,
	};
	VkCommandBuffer cmd;
	r = vkAllocateCommandBuffers(dev, &cb_ai, &cmd);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkAllocateCommandBuffers failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	VkCommandBufferBeginInfo begin_info = {
		.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
	};
	vkBeginCommandBuffer(cmd, &begin_info);

	/* Clear to black so any pixel that later reads as non-black can only
	 * have come from real rasterized/shaded triangle coverage. */
	VkClearValue clear_value = { .color = { .float32 = { 0.0f, 0.0f, 0.0f, 1.0f } } };
	VkRenderPassBeginInfo rp_begin = {
		.sType = VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
		.renderPass = render_pass,
		.framebuffer = framebuffer,
		.renderArea = { .offset = { 0, 0 }, .extent = { IMG_W, IMG_H } },
		.clearValueCount = 1,
		.pClearValues = &clear_value,
	};
	vkCmdBeginRenderPass(cmd, &rp_begin, VK_SUBPASS_CONTENTS_INLINE);
	vkCmdBindPipeline(cmd, VK_PIPELINE_BIND_POINT_GRAPHICS, pipeline);
	vkCmdDraw(cmd, 3, 1, 0, 0);
	vkCmdEndRenderPass(cmd);

	VkBufferImageCopy region = {
		.imageSubresource = {
			.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT,
			.layerCount = 1,
		},
		.imageExtent = { IMG_W, IMG_H, 1 },
	};
	vkCmdCopyImageToBuffer(cmd, image, VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
				readback_buf, 1, &region);

	VkBufferMemoryBarrier copy_to_host = {
		.sType = VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER,
		.srcAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT,
		.dstAccessMask = VK_ACCESS_HOST_READ_BIT,
		.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED,
		.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED,
		.buffer = readback_buf,
		.offset = 0,
		.size = VK_WHOLE_SIZE,
	};
	vkCmdPipelineBarrier(cmd,
			     VK_PIPELINE_STAGE_TRANSFER_BIT,
			     VK_PIPELINE_STAGE_HOST_BIT,
			     0, 0, NULL, 1, &copy_to_host, 0, NULL);

	r = vkEndCommandBuffer(cmd);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkEndCommandBuffer failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	VkSubmitInfo submit = {
		.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO,
		.commandBufferCount = 1,
		.pCommandBuffers = &cmd,
	};
	put("vk-triangle: submitting real triangle draw to the GPU queue\n");
	r = vkQueueSubmit(queue, 1, &submit, VK_NULL_HANDLE);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkQueueSubmit failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	r = vkQueueWaitIdle(queue);
	put("vk-triangle: vkQueueWaitIdle result=");
	put_hex((unsigned long)(long)r);
	put("\n");
	if (r != VK_SUCCESS)
		return 1;

	/* --- Read back and verify on the CPU side ---
	 * pixel[0] (top-left corner, (0,0)) is outside the triangle
	 * (vertices at NDC y=-0.8 and y=0.8; the corner's NDC is ~(-0.98,
	 * -0.98), above/outside the top edge) -- must stay the black clear
	 * color. The center pixel sits inside the triangle (its centroid is
	 * near NDC (0,-0.27)) and must show a real interpolated, non-black
	 * color -- the only way that can happen is if the vertex shader,
	 * rasterizer and fragment shader all really ran. */
	void *mapped;
	r = vkMapMemory(dev, buf_mem, 0, buf_size, 0, &mapped);
	if (r != VK_SUCCESS) {
		put("vk-triangle: vkMapMemory failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	unsigned char *px = (unsigned char *)mapped;
	put("vk-triangle: corner pixel[0,0] (expect background) = R=");
	put_dec(px[0]);
	put(" G=");
	put_dec(px[1]);
	put(" B=");
	put_dec(px[2]);
	put(" A=");
	put_dec(px[3]);
	put("\n");

	unsigned char *mid_px = px + (((IMG_H / 2) * IMG_W + (IMG_W / 2)) * 4);
	put("vk-triangle: center pixel (expect rasterized triangle) = R=");
	put_dec(mid_px[0]);
	put(" G=");
	put_dec(mid_px[1]);
	put(" B=");
	put_dec(mid_px[2]);
	put(" A=");
	put_dec(mid_px[3]);
	put("\n");

	int corner_is_background = px[0] < 40 && px[1] < 40 && px[2] < 40 && px[3] > 200;
	int center_sum = (int)mid_px[0] + (int)mid_px[1] + (int)mid_px[2];
	int center_is_rasterized = center_sum > 80 && mid_px[3] > 200;

	vkUnmapMemory(dev, buf_mem);

	if (corner_is_background && center_is_rasterized) {
		put("vk-triangle: PASS -- real rasterized/shaded triangle verified on CPU (3D pipeline works)\n");
	} else {
		put("vk-triangle: FAIL -- corner/center pixels do not match expected rasterization\n");
		return 1;
	}

	put("vk-triangle: done\n");
	return 0;
}
