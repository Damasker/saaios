/* SaaiOS panther GPU compositor helper.
 *
 * saai-displayd remains a static-musl Wayland/DRM process.  Google's Mali
 * UMD is bionic-only, so this small bionic child owns Vulkan and imports the
 * two DRM dumb-buffer dma-buf fds inherited from its parent.  A compact
 * stdin/stdout protocol mirrors HardwareOutput::fill/blit/present:
 *
 *   command[8] = magic, op, slot, width, height, stride, payload_len, arg
 *   BEGIN (1): acquire slot from KMS and clear to packed 0x00RRGGBB `arg`
 *   BLIT  (2): read payload_len bytes and GPU-copy them at (0, 0)
 *   END   (3): release slot back to KMS
 *
 * The helper replies with one 'K' after each completed command and emits one
 * initial 'R' only after both scanout buffers have been imported by Vulkan.
 */
#include <vulkan/vulkan.h>
#include <drm_fourcc.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <dlfcn.h>

#define PROTOCOL_MAGIC 0x53475055u /* "UGPS" in little-endian memory */
#define OP_BEGIN 1u
#define OP_BLIT 2u
#define OP_END 3u
#define HWVULKAN_DEVICE_0 "vk0"

struct command {
	uint32_t magic;
	uint32_t op;
	uint32_t slot;
	uint32_t width;
	uint32_t height;
	uint32_t stride;
	uint32_t payload_len;
	uint32_t arg;
};

struct hw_module_t;
struct hw_device_t;

struct hw_module_methods_t {
	int (*open)(const struct hw_module_t *, const char *,
		    struct hw_device_t **);
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
	int (*close)(struct hw_device_t *);
};

typedef VkResult (*PFN_vkEnumerateInstanceExtensionProperties_t)(
	const char *, uint32_t *, VkExtensionProperties *);

struct hwvulkan_device_t {
	struct hw_device_t common;
	PFN_vkEnumerateInstanceExtensionProperties_t
		EnumerateInstanceExtensionProperties;
	PFN_vkCreateInstance CreateInstance;
	PFN_vkGetInstanceProcAddr GetInstanceProcAddr;
};

struct imported_image {
	VkImage image;
	VkDeviceMemory memory;
};

static PFN_vkGetInstanceProcAddr g_gipa;
static VkInstance g_instance;
static VkDevice g_device;
static VkQueue g_queue;
static uint32_t g_queue_family;
static VkCommandPool g_command_pool;
static VkCommandBuffer g_command;
static VkBuffer g_staging_buffer;
static VkDeviceMemory g_staging_memory;
static void *g_staging_mapping;
static VkDeviceSize g_staging_size;
static struct imported_image g_images[2];
static int g_active_slot = -1;

#define DECLARE(fn) static PFN_##fn p_##fn
DECLARE(vkEnumeratePhysicalDevices);
DECLARE(vkGetPhysicalDeviceQueueFamilyProperties);
DECLARE(vkGetPhysicalDeviceMemoryProperties);
DECLARE(vkCreateDevice);
DECLARE(vkGetDeviceQueue);
DECLARE(vkCreateImage);
DECLARE(vkGetImageMemoryRequirements);
DECLARE(vkAllocateMemory);
DECLARE(vkBindImageMemory);
DECLARE(vkCreateBuffer);
DECLARE(vkGetBufferMemoryRequirements);
DECLARE(vkBindBufferMemory);
DECLARE(vkMapMemory);
DECLARE(vkCreateCommandPool);
DECLARE(vkAllocateCommandBuffers);
DECLARE(vkResetCommandPool);
DECLARE(vkBeginCommandBuffer);
DECLARE(vkCmdPipelineBarrier);
DECLARE(vkCmdClearColorImage);
DECLARE(vkCmdCopyBufferToImage);
DECLARE(vkEndCommandBuffer);
DECLARE(vkQueueSubmit);
DECLARE(vkQueueWaitIdle);
DECLARE(vkGetMemoryFdPropertiesKHR);

#define LOAD(fn) \
	do { \
		p_##fn = (PFN_##fn)g_gipa(g_instance, #fn); \
		if (!p_##fn) { \
			fprintf(stderr, "saai-gpu-compositor: missing %s\n", #fn); \
			return -1; \
		} \
	} while (0)

static int read_full(int fd, void *buffer, size_t size)
{
	unsigned char *cursor = buffer;
	while (size > 0) {
		ssize_t count = read(fd, cursor, size);
		if (count == 0)
			return 0;
		if (count < 0) {
			if (errno == EINTR)
				continue;
			return -1;
		}
		cursor += count;
		size -= (size_t)count;
	}
	return 1;
}

static int write_full(int fd, const void *buffer, size_t size)
{
	const unsigned char *cursor = buffer;
	while (size > 0) {
		ssize_t count = write(fd, cursor, size);
		if (count < 0) {
			if (errno == EINTR)
				continue;
			return -1;
		}
		cursor += count;
		size -= (size_t)count;
	}
	return 0;
}

static uint32_t find_memory_type(const VkPhysicalDeviceMemoryProperties *mp,
				 uint32_t bits,
				 VkMemoryPropertyFlags wanted)
{
	for (uint32_t i = 0; i < mp->memoryTypeCount; ++i) {
		if ((bits & (1u << i)) &&
		    (mp->memoryTypes[i].propertyFlags & wanted) == wanted)
			return i;
	}
	return UINT32_MAX;
}

static int begin_commands(void)
{
	VkResult result = p_vkResetCommandPool(g_device, g_command_pool, 0);
	if (result != VK_SUCCESS)
		return -1;
	VkCommandBufferBeginInfo begin = {
		.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
		.flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
	};
	return p_vkBeginCommandBuffer(g_command, &begin) == VK_SUCCESS ? 0 : -1;
}

static int submit_commands(void)
{
	if (p_vkEndCommandBuffer(g_command) != VK_SUCCESS)
		return -1;
	VkSubmitInfo submit = {
		.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO,
		.commandBufferCount = 1,
		.pCommandBuffers = &g_command,
	};
	VkResult result = p_vkQueueSubmit(g_queue, 1, &submit, VK_NULL_HANDLE);
	if (result == VK_SUCCESS)
		result = p_vkQueueWaitIdle(g_queue);
	return result == VK_SUCCESS ? 0 : -1;
}

static VkImageMemoryBarrier image_barrier(VkImage image)
{
	VkImageMemoryBarrier barrier = {
		.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
		.image = image,
		.subresourceRange = {
			.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT,
			.levelCount = 1,
			.layerCount = 1,
		},
	};
	return barrier;
}

static int gpu_begin(uint32_t slot, uint32_t color)
{
	if (slot >= 2 || g_active_slot >= 0 || begin_commands() < 0)
		return -1;
	VkImageMemoryBarrier acquire = image_barrier(g_images[slot].image);
	acquire.srcAccessMask = 0;
	acquire.dstAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
	acquire.oldLayout = VK_IMAGE_LAYOUT_UNDEFINED;
	acquire.newLayout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
	acquire.srcQueueFamilyIndex = VK_QUEUE_FAMILY_FOREIGN_EXT;
	acquire.dstQueueFamilyIndex = g_queue_family;
	p_vkCmdPipelineBarrier(g_command,
		VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,
		VK_PIPELINE_STAGE_TRANSFER_BIT, 0,
		0, NULL, 0, NULL, 1, &acquire);
	VkClearColorValue clear = { .float32 = {
		((color >> 16) & 0xffu) / 255.0f,
		((color >> 8) & 0xffu) / 255.0f,
		(color & 0xffu) / 255.0f,
		1.0f,
	} };
	VkImageSubresourceRange range = {
		.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT,
		.levelCount = 1,
		.layerCount = 1,
	};
	p_vkCmdClearColorImage(g_command, g_images[slot].image,
		VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, &clear, 1, &range);
	if (submit_commands() < 0)
		return -1;
	g_active_slot = (int)slot;
	return 0;
}

static int gpu_blit(uint32_t width, uint32_t height, uint32_t stride)
{
	if (g_active_slot < 0 || width == 0 || height == 0 ||
	    stride < width * 4 || begin_commands() < 0)
		return -1;
	VkBufferMemoryBarrier staging_ready = {
		.sType = VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER,
		.srcAccessMask = VK_ACCESS_HOST_WRITE_BIT,
		.dstAccessMask = VK_ACCESS_TRANSFER_READ_BIT,
		.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED,
		.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED,
		.buffer = g_staging_buffer,
		.offset = 0,
		.size = VK_WHOLE_SIZE,
	};
	p_vkCmdPipelineBarrier(g_command,
		VK_PIPELINE_STAGE_HOST_BIT,
		VK_PIPELINE_STAGE_TRANSFER_BIT, 0,
		0, NULL, 1, &staging_ready, 0, NULL);
	VkBufferImageCopy copy = {
		.bufferOffset = 0,
		.bufferRowLength = stride / 4,
		.bufferImageHeight = height,
		.imageSubresource = {
			.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT,
			.layerCount = 1,
		},
		.imageExtent = { width, height, 1 },
	};
	p_vkCmdCopyBufferToImage(g_command, g_staging_buffer,
		g_images[g_active_slot].image,
		VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, 1, &copy);
	return submit_commands();
}

static int gpu_end(uint32_t slot)
{
	if (slot >= 2 || g_active_slot != (int)slot || begin_commands() < 0)
		return -1;
	VkImageMemoryBarrier release = image_barrier(g_images[slot].image);
	release.srcAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
	release.dstAccessMask = 0;
	release.oldLayout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
	release.newLayout = VK_IMAGE_LAYOUT_GENERAL;
	release.srcQueueFamilyIndex = g_queue_family;
	release.dstQueueFamilyIndex = VK_QUEUE_FAMILY_FOREIGN_EXT;
	p_vkCmdPipelineBarrier(g_command,
		VK_PIPELINE_STAGE_TRANSFER_BIT,
		VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, 0,
		0, NULL, 0, NULL, 1, &release);
	if (submit_commands() < 0)
		return -1;
	g_active_slot = -1;
	return 0;
}

static int import_image(const VkPhysicalDeviceMemoryProperties *memory_properties,
			uint32_t width, uint32_t height, int dma_fd,
			struct imported_image *output)
{
	uint64_t linear_modifier = DRM_FORMAT_MOD_LINEAR;
	VkImageDrmFormatModifierListCreateInfoEXT modifiers = {
		.sType = VK_STRUCTURE_TYPE_IMAGE_DRM_FORMAT_MODIFIER_LIST_CREATE_INFO_EXT,
		.drmFormatModifierCount = 1,
		.pDrmFormatModifiers = &linear_modifier,
	};
	VkExternalMemoryImageCreateInfo external = {
		.sType = VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO,
		.pNext = &modifiers,
		.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT,
	};
	VkImageCreateInfo image_info = {
		.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
		.pNext = &external,
		.imageType = VK_IMAGE_TYPE_2D,
		.format = VK_FORMAT_B8G8R8A8_UNORM,
		.extent = { width, height, 1 },
		.mipLevels = 1,
		.arrayLayers = 1,
		.samples = VK_SAMPLE_COUNT_1_BIT,
		.tiling = VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT,
		.usage = VK_IMAGE_USAGE_TRANSFER_DST_BIT,
		.sharingMode = VK_SHARING_MODE_EXCLUSIVE,
		.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED,
	};
	VkResult result = p_vkCreateImage(g_device, &image_info, NULL,
					  &output->image);
	if (result != VK_SUCCESS)
		return -1;

	VkMemoryRequirements requirements;
	p_vkGetImageMemoryRequirements(g_device, output->image, &requirements);
	VkMemoryFdPropertiesKHR fd_properties = {
		.sType = VK_STRUCTURE_TYPE_MEMORY_FD_PROPERTIES_KHR,
	};
	result = p_vkGetMemoryFdPropertiesKHR(
		g_device, VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT,
		dma_fd, &fd_properties);
	if (result != VK_SUCCESS)
		return -1;
	uint32_t memory_type = find_memory_type(
		memory_properties,
		requirements.memoryTypeBits & fd_properties.memoryTypeBits, 0);
	if (memory_type == UINT32_MAX)
		return -1;
	VkMemoryDedicatedAllocateInfo dedicated = {
		.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO,
		.image = output->image,
	};
	VkImportMemoryFdInfoKHR import = {
		.sType = VK_STRUCTURE_TYPE_IMPORT_MEMORY_FD_INFO_KHR,
		.pNext = &dedicated,
		.handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT,
		.fd = dup(dma_fd),
	};
	if (import.fd < 0)
		return -1;
	VkMemoryAllocateInfo allocation = {
		.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
		.pNext = &import,
		.allocationSize = requirements.size,
		.memoryTypeIndex = memory_type,
	};
	result = p_vkAllocateMemory(g_device, &allocation, NULL,
				    &output->memory);
	if (result != VK_SUCCESS) {
		close(import.fd);
		return -1;
	}
	return p_vkBindImageMemory(g_device, output->image,
				   output->memory, 0) == VK_SUCCESS ? 0 : -1;
}

static int initialize_vulkan(uint32_t width, uint32_t height, uint32_t pitch,
			     int dma_fd0, int dma_fd1)
{
	void *library = dlopen("/vendor/lib64/hw/vulkan.mali.so", RTLD_NOW);
	if (!library) {
		fprintf(stderr, "saai-gpu-compositor: dlopen failed: %s\n",
			dlerror());
		return -1;
	}
	struct hw_module_t *module = dlsym(library, "HMI");
	if (!module || !module->methods || !module->methods->open)
		return -1;
	struct hw_device_t *base_device = NULL;
	if (module->methods->open(module, HWVULKAN_DEVICE_0, &base_device) != 0 ||
	    !base_device)
		return -1;
	struct hwvulkan_device_t *vkdev =
		(struct hwvulkan_device_t *)base_device;
	g_gipa = vkdev->GetInstanceProcAddr;
	VkApplicationInfo application = {
		.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
		.pApplicationName = "saai-gpu-compositor",
		.apiVersion = VK_API_VERSION_1_1,
	};
	VkInstanceCreateInfo instance_info = {
		.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
		.pApplicationInfo = &application,
	};
	if (vkdev->CreateInstance(&instance_info, NULL, &g_instance) != VK_SUCCESS)
		return -1;

	LOAD(vkEnumeratePhysicalDevices);
	LOAD(vkGetPhysicalDeviceQueueFamilyProperties);
	LOAD(vkGetPhysicalDeviceMemoryProperties);
	LOAD(vkCreateDevice);
	LOAD(vkGetDeviceQueue);
	LOAD(vkCreateImage);
	LOAD(vkGetImageMemoryRequirements);
	LOAD(vkAllocateMemory);
	LOAD(vkBindImageMemory);
	LOAD(vkCreateBuffer);
	LOAD(vkGetBufferMemoryRequirements);
	LOAD(vkBindBufferMemory);
	LOAD(vkMapMemory);
	LOAD(vkCreateCommandPool);
	LOAD(vkAllocateCommandBuffers);
	LOAD(vkResetCommandPool);
	LOAD(vkBeginCommandBuffer);
	LOAD(vkCmdPipelineBarrier);
	LOAD(vkCmdClearColorImage);
	LOAD(vkCmdCopyBufferToImage);
	LOAD(vkEndCommandBuffer);
	LOAD(vkQueueSubmit);
	LOAD(vkQueueWaitIdle);
	LOAD(vkGetMemoryFdPropertiesKHR);

	uint32_t physical_count = 1;
	VkPhysicalDevice physical;
	if (p_vkEnumeratePhysicalDevices(g_instance, &physical_count, &physical) !=
		VK_SUCCESS || physical_count == 0)
		return -1;
	uint32_t family_count = 8;
	VkQueueFamilyProperties families[8];
	p_vkGetPhysicalDeviceQueueFamilyProperties(physical, &family_count,
						  families);
	g_queue_family = UINT32_MAX;
	for (uint32_t i = 0; i < family_count; ++i) {
		if (families[i].queueFlags & VK_QUEUE_GRAPHICS_BIT) {
			g_queue_family = i;
			break;
		}
	}
	if (g_queue_family == UINT32_MAX)
		return -1;

	const char *extensions[] = {
		VK_KHR_EXTERNAL_MEMORY_EXTENSION_NAME,
		VK_KHR_EXTERNAL_MEMORY_FD_EXTENSION_NAME,
		VK_EXT_EXTERNAL_MEMORY_DMA_BUF_EXTENSION_NAME,
		VK_EXT_IMAGE_DRM_FORMAT_MODIFIER_EXTENSION_NAME,
		VK_KHR_DEDICATED_ALLOCATION_EXTENSION_NAME,
		VK_KHR_GET_MEMORY_REQUIREMENTS_2_EXTENSION_NAME,
		VK_KHR_BIND_MEMORY_2_EXTENSION_NAME,
		VK_EXT_QUEUE_FAMILY_FOREIGN_EXTENSION_NAME,
	};
	float priority = 1.0f;
	VkDeviceQueueCreateInfo queue_info = {
		.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
		.queueFamilyIndex = g_queue_family,
		.queueCount = 1,
		.pQueuePriorities = &priority,
	};
	VkDeviceCreateInfo device_info = {
		.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
		.queueCreateInfoCount = 1,
		.pQueueCreateInfos = &queue_info,
		.enabledExtensionCount = sizeof(extensions) / sizeof(extensions[0]),
		.ppEnabledExtensionNames = extensions,
	};
	if (p_vkCreateDevice(physical, &device_info, NULL, &g_device) !=
		VK_SUCCESS)
		return -1;
	p_vkGetDeviceQueue(g_device, g_queue_family, 0, &g_queue);

	VkPhysicalDeviceMemoryProperties memory_properties;
	p_vkGetPhysicalDeviceMemoryProperties(physical, &memory_properties);
	if (import_image(&memory_properties, width, height, dma_fd0,
			 &g_images[0]) < 0 ||
	    import_image(&memory_properties, width, height, dma_fd1,
			 &g_images[1]) < 0)
		return -1;

	g_staging_size = (VkDeviceSize)pitch * height;
	VkBufferCreateInfo buffer_info = {
		.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
		.size = g_staging_size,
		.usage = VK_BUFFER_USAGE_TRANSFER_SRC_BIT,
		.sharingMode = VK_SHARING_MODE_EXCLUSIVE,
	};
	if (p_vkCreateBuffer(g_device, &buffer_info, NULL, &g_staging_buffer) !=
		VK_SUCCESS)
		return -1;
	VkMemoryRequirements staging_requirements;
	p_vkGetBufferMemoryRequirements(g_device, g_staging_buffer,
					&staging_requirements);
	uint32_t staging_type = find_memory_type(
		&memory_properties, staging_requirements.memoryTypeBits,
		VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT |
		VK_MEMORY_PROPERTY_HOST_COHERENT_BIT);
	if (staging_type == UINT32_MAX)
		return -1;
	VkMemoryAllocateInfo staging_allocation = {
		.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
		.allocationSize = staging_requirements.size,
		.memoryTypeIndex = staging_type,
	};
	if (p_vkAllocateMemory(g_device, &staging_allocation, NULL,
			       &g_staging_memory) != VK_SUCCESS ||
	    p_vkBindBufferMemory(g_device, g_staging_buffer,
				 g_staging_memory, 0) != VK_SUCCESS ||
	    p_vkMapMemory(g_device, g_staging_memory, 0, g_staging_size, 0,
			  &g_staging_mapping) != VK_SUCCESS)
		return -1;

	VkCommandPoolCreateInfo pool_info = {
		.sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
		.flags = VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
		.queueFamilyIndex = g_queue_family,
	};
	if (p_vkCreateCommandPool(g_device, &pool_info, NULL,
				  &g_command_pool) != VK_SUCCESS)
		return -1;
	VkCommandBufferAllocateInfo command_info = {
		.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
		.commandPool = g_command_pool,
		.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY,
		.commandBufferCount = 1,
	};
	if (p_vkAllocateCommandBuffers(g_device, &command_info,
				       &g_command) != VK_SUCCESS)
		return -1;
	return 0;
}

static int parse_u32(const char *text, uint32_t *value)
{
	char *end = NULL;
	unsigned long parsed = strtoul(text, &end, 10);
	if (!text[0] || !end || *end || parsed > UINT32_MAX)
		return -1;
	*value = (uint32_t)parsed;
	return 0;
}

int main(int argc, char **argv)
{
	if (argc != 6) {
		fprintf(stderr,
			"usage: %s WIDTH HEIGHT PITCH DMA_FD0 DMA_FD1\n", argv[0]);
		return 64;
	}
	uint32_t width, height, pitch, fd0, fd1;
	if (parse_u32(argv[1], &width) < 0 ||
	    parse_u32(argv[2], &height) < 0 ||
	    parse_u32(argv[3], &pitch) < 0 ||
	    parse_u32(argv[4], &fd0) < 0 ||
	    parse_u32(argv[5], &fd1) < 0 ||
	    width == 0 || height == 0 || pitch < width * 4) {
		fprintf(stderr, "saai-gpu-compositor: invalid arguments\n");
		return 64;
	}
	if (initialize_vulkan(width, height, pitch, (int)fd0, (int)fd1) < 0) {
		fprintf(stderr, "saai-gpu-compositor: Vulkan initialization failed\n");
		return 70;
	}
	fprintf(stderr,
		"saai-gpu-compositor: ready %ux%u pitch=%u (zero-copy dma-buf)\n",
		width, height, pitch);
	if (write_full(STDOUT_FILENO, "R", 1) < 0)
		return 71;

	for (;;) {
		struct command command;
		int read_result = read_full(STDIN_FILENO, &command, sizeof(command));
		if (read_result == 0)
			return 0;
		if (read_result < 0 || command.magic != PROTOCOL_MAGIC) {
			fprintf(stderr, "saai-gpu-compositor: protocol read failed\n");
			return 72;
		}
		int result = -1;
		switch (command.op) {
		case OP_BEGIN:
			result = gpu_begin(command.slot, command.arg);
			break;
		case OP_BLIT:
			if (command.payload_len > g_staging_size ||
			    command.payload_len != command.stride * command.height ||
			    command.width > width || command.height > height) {
				fprintf(stderr,
					"saai-gpu-compositor: invalid blit geometry\n");
				return 73;
			}
			if (read_full(STDIN_FILENO, g_staging_mapping,
				      command.payload_len) != 1)
				return 74;
			result = gpu_blit(command.width, command.height,
					  command.stride);
			break;
		case OP_END:
			result = gpu_end(command.slot);
			break;
		default:
			fprintf(stderr, "saai-gpu-compositor: unknown operation %u\n",
				command.op);
			return 75;
		}
		if (result < 0) {
			fprintf(stderr,
				"saai-gpu-compositor: GPU operation %u failed\n",
				command.op);
			return 76;
		}
		if (write_full(STDOUT_FILENO, "K", 1) < 0)
			return 77;
	}
}
