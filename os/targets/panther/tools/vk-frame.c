/* ADR-024 continued: real frame submit. Reuses the exact bootstrap
 * proven in vk_probe.c (bypass libvulkan.so's broken HAL discovery,
 * talk to the real Mali UMD's hwvulkan_device_t directly) up through
 * vkCreateDevice, then does a genuine offscreen render: create a
 * color image, a render pass that clears it to a known color, record
 * and submit a real command buffer, copy the rendered image to a
 * host-visible buffer, and verify the pixel value on the CPU side --
 * proof that real GPU work executed, not just that objects were
 * created.
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

static PFN_vkGetInstanceProcAddr g_gipa;
static VkInstance g_instance;

#define LOAD(fn) PFN_##fn fn = (PFN_##fn)g_gipa(g_instance, #fn); \
	do { \
		if (!fn) { \
			put("vk-frame: FATAL missing " #fn "\n"); \
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

/* Using the real NDK crtbegin_dynamic.o/crtend_android.o for startup
 * this time (see build notes) instead of a hand-rolled _start: the
 * Mali UMD's internal fence-completion synchronization needs a
 * properly initialized bionic TLS/pthread_internal_t for the main
 * thread (and possibly to spawn its own worker thread), which a bare
 * "bl main" entry point never sets up. Real crt calls main(argc,argv,envp). */
int main(int argc, char **argv, char **envp)
{
	(void)argc;
	(void)argv;
	(void)envp;
	put("vk-frame: dlopen(/vendor/lib64/hw/vulkan.mali.so)\n");
	void *h = dlopen("/vendor/lib64/hw/vulkan.mali.so", RTLD_NOW);
	if (!h) {
		put("vk-frame: dlopen FAILED: ");
		put(dlerror());
		put("\n");
		return 1;
	}

	struct hw_module_t *module = (struct hw_module_t *)dlsym(h, "HMI");
	if (!module) {
		put("vk-frame: dlsym(HMI) FAILED\n");
		return 1;
	}

	struct hw_device_t *device_h = NULL;
	int ret = module->methods->open(module, HWVULKAN_DEVICE_0, &device_h);
	if (ret != 0 || !device_h) {
		put("vk-frame: open() failed\n");
		return 1;
	}

	struct hwvulkan_device_t *vkdev = (struct hwvulkan_device_t *)device_h;
	if (!vkdev->CreateInstance || !vkdev->GetInstanceProcAddr) {
		put("vk-frame: NULL CreateInstance/GetInstanceProcAddr\n");
		return 1;
	}
	g_gipa = vkdev->GetInstanceProcAddr;

	VkApplicationInfo app_info = {
		.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
		.pApplicationName = "vk-frame",
		.apiVersion = VK_API_VERSION_1_0,
	};
	VkInstanceCreateInfo inst_info = {
		.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
		.pApplicationInfo = &app_info,
	};
	VkResult r = vkdev->CreateInstance(&inst_info, NULL, &g_instance);
	if (r != VK_SUCCESS) {
		put("vk-frame: CreateInstance failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	put("vk-frame: VkInstance created\n");

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
	LOAD(vkCreateCommandPool);
	LOAD(vkAllocateCommandBuffers);
	LOAD(vkBeginCommandBuffer);
	LOAD(vkCmdBeginRenderPass);
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
	put("vk-frame: all function pointers loaded\n");

	uint32_t pd_count = 1;
	VkPhysicalDevice phys;
	r = vkEnumeratePhysicalDevices(g_instance, &pd_count, &phys);
	if (r != VK_SUCCESS || pd_count == 0) {
		put("vk-frame: no physical device\n");
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
		put("vk-frame: no graphics queue family\n");
		return 1;
	}
	put("vk-frame: using queue family ");
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
		put("vk-frame: vkCreateDevice failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	put("vk-frame: VkDevice created\n");

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
		put("vk-frame: vkCreateImage failed: ");
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
		put("vk-frame: image vkAllocateMemory failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	vkBindImageMemory(dev, image, image_mem, 0);
	put("vk-frame: color image created and bound\n");

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
		put("vk-frame: vkCreateImageView failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	/* --- Render pass: single color attachment, clear to red, then
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
	/* The implicit subpass->external dependency does not make the color
	 * attachment writes available to the transfer stage that immediately
	 * copies the image below.  On the real Mali driver the queue still
	 * completes successfully, but the copy can legally observe the old
	 * contents (all zeroes).  Make that dependency explicit. */
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
		put("vk-frame: vkCreateRenderPass failed: ");
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
		put("vk-frame: vkCreateFramebuffer failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	put("vk-frame: render pass + framebuffer created\n");

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
		put("vk-frame: vkCreateBuffer failed: ");
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
		put("vk-frame: no HOST_VISIBLE|HOST_COHERENT memory type\n");
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
		put("vk-frame: buffer vkAllocateMemory failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	vkBindBufferMemory(dev, readback_buf, buf_mem, 0);
	/* Poison the destination before submission so an omitted copy cannot be
	 * mistaken for a legitimately rendered transparent-black frame. */
	void *initial_map;
	r = vkMapMemory(dev, buf_mem, 0, buf_size, 0, &initial_map);
	if (r != VK_SUCCESS) {
		put("vk-frame: initial vkMapMemory failed\n");
		return 1;
	}
	memset(initial_map, 0x11, (unsigned long)buf_size);
	vkUnmapMemory(dev, buf_mem);
	put("vk-frame: readback buffer created and bound\n");

	/* --- Command pool + buffer --- */
	VkCommandPoolCreateInfo pool_ci = {
		.sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
		.queueFamilyIndex = qfi,
	};
	VkCommandPool cmd_pool;
	r = vkCreateCommandPool(dev, &pool_ci, NULL, &cmd_pool);
	if (r != VK_SUCCESS) {
		put("vk-frame: vkCreateCommandPool failed: ");
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
		put("vk-frame: vkAllocateCommandBuffers failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	VkCommandBufferBeginInfo begin_info = {
		.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
	};
	vkBeginCommandBuffer(cmd, &begin_info);

	/* Clear to solid red (R=1.0, G=0.0, B=0.0, A=1.0). */
	VkClearValue clear_value = { .color = { .float32 = { 1.0f, 0.0f, 0.0f, 1.0f } } };
	VkRenderPassBeginInfo rp_begin = {
		.sType = VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
		.renderPass = render_pass,
		.framebuffer = framebuffer,
		.renderArea = { .offset = { 0, 0 }, .extent = { IMG_W, IMG_H } },
		.clearValueCount = 1,
		.pClearValues = &clear_value,
	};
	vkCmdBeginRenderPass(cmd, &rp_begin, VK_SUBPASS_CONTENTS_INLINE);
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

	/* Queue completion alone is not a memory dependency.  Publish transfer
	 * writes to subsequent host reads before ending the command buffer. */
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
		put("vk-frame: vkEndCommandBuffer failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}

	VkSubmitInfo submit = {
		.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO,
		.commandBufferCount = 1,
		.pCommandBuffers = &cmd,
	};
	put("vk-frame: submitting real command buffer to the GPU queue\n");
	r = vkQueueSubmit(queue, 1, &submit, VK_NULL_HANDLE);
	if (r != VK_SUCCESS) {
		put("vk-frame: vkQueueSubmit failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	r = vkQueueWaitIdle(queue);
	put("vk-frame: vkQueueWaitIdle result=");
	put_hex((unsigned long)(long)r);
	put("\n");
	if (r != VK_SUCCESS)
		return 1;

	/* --- Read back and verify on the CPU side --- */
	void *mapped;
	r = vkMapMemory(dev, buf_mem, 0, buf_size, 0, &mapped);
	if (r != VK_SUCCESS) {
		put("vk-frame: vkMapMemory failed: ");
		put_hex((unsigned long)(long)r);
		put("\n");
		return 1;
	}
	unsigned char *px = (unsigned char *)mapped;
	put("vk-frame: pixel[0] = R=");
	put_dec(px[0]);
	put(" G=");
	put_dec(px[1]);
	put(" B=");
	put_dec(px[2]);
	put(" A=");
	put_dec(px[3]);
	put("\n");

	unsigned char *mid_px = px + (((IMG_H / 2) * IMG_W + (IMG_W / 2)) * 4);
	put("vk-frame: pixel[center] = R=");
	put_dec(mid_px[0]);
	put(" G=");
	put_dec(mid_px[1]);
	put(" B=");
	put_dec(mid_px[2]);
	put(" A=");
	put_dec(mid_px[3]);
	put("\n");

	int pixels_are_red =
		px[0] > 200 && px[1] < 50 && px[2] < 50 && px[3] > 200 &&
		mid_px[0] > 200 && mid_px[1] < 50 && mid_px[2] < 50;
	vkUnmapMemory(dev, buf_mem);

	if (pixels_are_red) {
		put("vk-frame: PASS -- real GPU-rendered red frame verified on CPU\n");
	} else {
		put("vk-frame: FAIL -- pixel values do not match expected clear color\n");
		return 1;
	}

	put("vk-frame: done\n");
	return 0;
}
