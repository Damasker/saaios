/* SaaiOS panther GPU compositor helper.
 *
 * saai-displayd remains a static-musl Wayland/DRM process.  Google's Mali
 * UMD is bionic-only, so this small bionic child owns Vulkan and imports the
 * two DRM dumb-buffer dma-buf fds inherited from its parent as linear Vulkan
 * buffers.  A compact
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
#include <poll.h>
#include <sys/socket.h>
#include <unistd.h>
#include <dlfcn.h>

#define PROTOCOL_MAGIC 0x53475055u /* "UGPS" in little-endian memory */
#define OP_BEGIN 1u
#define OP_BLIT 2u
#define OP_END 3u
#define OP_DMABUF_BLIT 4u
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

struct imported_buffer {
	VkBuffer buffer;
	VkDeviceMemory memory;
};

static PFN_vkGetInstanceProcAddr g_gipa;
static VkInstance g_instance;
static VkDevice g_device;
static VkQueue g_queue;
static uint32_t g_queue_family;
static VkCommandPool g_command_pool;
static VkCommandBuffer g_command;
static VkFence g_fence;
static VkPhysicalDeviceMemoryProperties g_memory_properties;
static VkBuffer g_staging_buffer;
static VkDeviceMemory g_staging_memory;
static void *g_staging_mapping;
static VkDeviceSize g_staging_size;
static struct imported_buffer g_scanout[2];
static int g_active_slot = -1;
/* ADR-024 continued: one-submit-per-frame batching. Every OP_DMABUF_BLIT
 * between BEGIN and END records into BEGIN's still-open command buffer
 * (own dedicated imported VkBuffer/memory per call -- safe to keep
 * recording without waiting) instead of submitting and waiting
 * individually; the actual VkBuffer/memory for each import can only be
 * destroyed once the GPU has genuinely finished reading it, so END defers
 * that until after its own (now frame-wide) submit_commands() returns. */
#define MAX_PENDING_IMPORTS 8
static struct imported_buffer g_pending_imports[MAX_PENDING_IMPORTS];
static int g_pending_import_count;
/* Set after a BLIT records a copy from the single shared g_staging_buffer
 * into the still-open batch. A second BLIT this frame would overwrite
 * g_staging_buffer before the GPU has actually read the first one's
 * bytes (nothing has submitted it yet under batching) -- gpu_blit()
 * checks this to flush (submit + wait once, exactly like every op used
 * to before this batching existed) before recording a second one. */
static int g_batch_pending_staging_blit;

#define DECLARE(fn) static PFN_##fn p_##fn
DECLARE(vkEnumeratePhysicalDevices);
DECLARE(vkGetPhysicalDeviceQueueFamilyProperties);
DECLARE(vkGetPhysicalDeviceMemoryProperties);
DECLARE(vkCreateDevice);
DECLARE(vkGetDeviceQueue);
DECLARE(vkAllocateMemory);
DECLARE(vkFreeMemory);
DECLARE(vkCreateBuffer);
DECLARE(vkDestroyBuffer);
DECLARE(vkGetBufferMemoryRequirements);
DECLARE(vkBindBufferMemory);
DECLARE(vkMapMemory);
DECLARE(vkCreateCommandPool);
DECLARE(vkAllocateCommandBuffers);
DECLARE(vkResetCommandPool);
DECLARE(vkBeginCommandBuffer);
DECLARE(vkCmdPipelineBarrier);
DECLARE(vkCmdFillBuffer);
DECLARE(vkCmdCopyBuffer);
DECLARE(vkEndCommandBuffer);
DECLARE(vkQueueSubmit);
DECLARE(vkQueueWaitIdle);
DECLARE(vkCreateFence);
DECLARE(vkDestroyFence);
DECLARE(vkWaitForFences);
DECLARE(vkResetFences);
DECLARE(vkGetMemoryFdPropertiesKHR);
DECLARE(vkGetFenceFdKHR);

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

static int receive_fd(int socket_fd)
{
	char byte;
	struct iovec iov = {
		.iov_base = &byte,
		.iov_len = sizeof(byte),
	};
	union {
		struct cmsghdr header;
		unsigned char bytes[CMSG_SPACE(sizeof(int))];
	} control;
	memset(&control, 0, sizeof(control));
	struct msghdr message = {
		.msg_iov = &iov,
		.msg_iovlen = 1,
		.msg_control = control.bytes,
		.msg_controllen = sizeof(control.bytes),
	};
	ssize_t received;
	do {
		received = recvmsg(socket_fd, &message, 0);
	} while (received < 0 && errno == EINTR);
	if (received != 1 || byte != 'D')
		return -1;
	for (struct cmsghdr *header = CMSG_FIRSTHDR(&message);
	     header != NULL; header = CMSG_NXTHDR(&message, header)) {
		if (header->cmsg_level == SOL_SOCKET &&
		    header->cmsg_type == SCM_RIGHTS &&
		    header->cmsg_len >= CMSG_LEN(sizeof(int))) {
			int fd;
			memcpy(&fd, CMSG_DATA(header), sizeof(fd));
			return fd;
		}
	}
	return -1;
}

/* ADR-024 continued: the other direction of receive_fd() above -- sends
 * this frame's exported fence syncfd to displayd (marker byte 'F', vs.
 * receive_fd()'s own 'D' for a client dma-buf fd coming the other way on
 * the same socket). displayd passes it straight into the DRM atomic
 * commit's IN_FENCE_FD plane property; the kernel does the actual wait,
 * not either process's CPU. */
static int send_fd(int socket_fd, int fd)
{
	char byte = 'F';
	struct iovec iov = {
		.iov_base = &byte,
		.iov_len = sizeof(byte),
	};
	union {
		struct cmsghdr header;
		unsigned char bytes[CMSG_SPACE(sizeof(int))];
	} control;
	memset(&control, 0, sizeof(control));
	struct msghdr message = {
		.msg_iov = &iov,
		.msg_iovlen = 1,
		.msg_control = control.bytes,
		.msg_controllen = CMSG_SPACE(sizeof(int)),
	};
	struct cmsghdr *header = CMSG_FIRSTHDR(&message);
	header->cmsg_level = SOL_SOCKET;
	header->cmsg_type = SCM_RIGHTS;
	header->cmsg_len = CMSG_LEN(sizeof(int));
	memcpy(CMSG_DATA(header), &fd, sizeof(fd));
	ssize_t sent;
	do {
		sent = sendmsg(socket_fd, &message, 0);
	} while (sent < 0 && errno == EINTR);
	return sent == 1 ? 0 : -1;
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
	/* Explicit per-submission fence instead of vkQueueWaitIdle: this
	 * only depends on THIS submission's completion, not on draining the
	 * entire queue (which would also serialize against any unrelated
	 * future work sharing g_queue). A bounded wait means a genuinely
	 * wedged GPU fails this blit loudly instead of hanging the
	 * compositor forever. */
	VkResult result = p_vkQueueSubmit(g_queue, 1, &submit, g_fence);
	if (result == VK_SUCCESS)
		result = p_vkWaitForFences(g_device, 1, &g_fence, VK_TRUE,
					   2000000000ULL);
	p_vkResetFences(g_device, 1, &g_fence);
	return result == VK_SUCCESS ? 0 : -1;
}

static VkBufferMemoryBarrier buffer_barrier(VkBuffer buffer)
{
	VkBufferMemoryBarrier barrier = {
		.sType = VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER,
		.buffer = buffer,
		.offset = 0,
		.size = VK_WHOLE_SIZE,
	};
	return barrier;
}

/* Defined further down (needs `import_buffer`'s error-cleanup call
 * site above it); forward-declared here so gpu_blit()'s mid-frame
 * flush and gpu_end()'s frame-end cleanup can call it. */
static void destroy_imported_buffer(struct imported_buffer *buffer);

static int gpu_begin(uint32_t slot, uint32_t color)
{
	if (slot >= 2 || g_active_slot >= 0 || begin_commands() < 0)
		return -1;
	VkBufferMemoryBarrier acquire = buffer_barrier(g_scanout[slot].buffer);
	acquire.srcAccessMask = 0;
	acquire.dstAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
	acquire.srcQueueFamilyIndex = VK_QUEUE_FAMILY_FOREIGN_EXT;
	acquire.dstQueueFamilyIndex = g_queue_family;
	p_vkCmdPipelineBarrier(g_command,
		VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,
		VK_PIPELINE_STAGE_TRANSFER_BIT, 0,
		0, NULL, 1, &acquire, 0, NULL);
	/* XRGB8888 on little-endian is byte order B,G,R,X. */
	uint32_t xrgb = color & 0x00ffffffu;
	p_vkCmdFillBuffer(g_command, g_scanout[slot].buffer,
			  0, VK_WHOLE_SIZE, xrgb);
	/* ADR-024 continued: no submit here -- every BLIT/DMABUF_BLIT this
	 * frame records into this same command buffer; gpu_end() submits
	 * once and waits once instead of once per operation (was 3+ full
	 * GPU submit+wait IPC round trips per frame before). */
	g_batch_pending_staging_blit = 0;
	g_active_slot = (int)slot;
	return 0;
}

static int gpu_blit(uint32_t width, uint32_t height, uint32_t stride,
		    uint32_t output_pitch)
{
	if (g_active_slot < 0 || width == 0 || height == 0 ||
	    stride < width * 4)
		return -1;
	if (g_batch_pending_staging_blit) {
		/* A second CPU-staged blit this frame -- flush what's
		 * recorded so far (submit + wait once) before overwriting
		 * g_staging_buffer, or the first blit's still-unsubmitted
		 * copy command would end up reading the second blit's
		 * bytes once the GPU actually executes it. */
		if (submit_commands() < 0)
			return -1;
		for (int i = 0; i < g_pending_import_count; ++i)
			destroy_imported_buffer(&g_pending_imports[i]);
		g_pending_import_count = 0;
		g_batch_pending_staging_blit = 0;
		if (begin_commands() < 0)
			return -1;
	}
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
	if (stride == output_pitch && width * 4 == output_pitch) {
		VkBufferCopy copy = {
			.srcOffset = 0,
			.dstOffset = 0,
			.size = (VkDeviceSize)stride * height,
		};
		p_vkCmdCopyBuffer(g_command, g_staging_buffer,
			g_scanout[g_active_slot].buffer, 1, &copy);
	} else {
		VkBufferCopy *rows = calloc(height, sizeof(*rows));
		if (!rows)
			return -1;
		for (uint32_t row = 0; row < height; ++row) {
			rows[row].srcOffset = (VkDeviceSize)row * stride;
			rows[row].dstOffset = (VkDeviceSize)row * output_pitch;
			rows[row].size = (VkDeviceSize)width * 4;
		}
		p_vkCmdCopyBuffer(g_command, g_staging_buffer,
			g_scanout[g_active_slot].buffer, height, rows);
		free(rows);
	}
	g_batch_pending_staging_blit = 1;
	return 0;
}

/* ADR-024 continued: full async submit. This is the ONE real
 * vkQueueSubmit for everything BEGIN/BLIT/DMABUF_BLIT recorded this
 * frame (or, if gpu_blit() had to flush mid-frame, for whatever it left
 * open afterward) -- but unlike the batching-only version, it does NOT
 * wait here. The fence was created with VkExportFenceCreateInfoKHR
 * (VK_EXTERNAL_FENCE_HANDLE_TYPE_SYNC_FD_BIT_KHR), so instead of
 * blocking this process (and, through the IPC round trip, displayd's
 * CPU thread) until the GPU finishes, `vkGetFenceFdKHR` exports it as a
 * syncfd the caller sends to displayd for the DRM atomic commit's
 * IN_FENCE_FD plane property -- the kernel itself waits before
 * scanning out, not any userspace CPU. Exporting also resets `g_fence`
 * to unsignaled per spec, ready for next frame's `vkQueueSubmit`
 * without an explicit `vkResetFences` call.
 *
 * `*out_fence_fd` is a new, distinct fd on success; the caller owns it
 * (must eventually close it, after sending a copy to displayd). This
 * function does NOT destroy this frame's `g_pending_imports` client
 * dma-buf buffers -- the caller must wait for the exported fence to
 * actually signal first (see OP_END's own comment in main()'s dispatch
 * loop for why that wait belongs there instead of here). */
static int gpu_end(uint32_t slot, int *out_fence_fd)
{
	if (slot >= 2 || g_active_slot != (int)slot)
		return -1;
	VkBufferMemoryBarrier release = buffer_barrier(g_scanout[slot].buffer);
	release.srcAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
	release.dstAccessMask = 0;
	release.srcQueueFamilyIndex = g_queue_family;
	release.dstQueueFamilyIndex = VK_QUEUE_FAMILY_FOREIGN_EXT;
	p_vkCmdPipelineBarrier(g_command,
		VK_PIPELINE_STAGE_TRANSFER_BIT,
		VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, 0,
		0, NULL, 1, &release, 0, NULL);
	if (p_vkEndCommandBuffer(g_command) != VK_SUCCESS)
		return -1;
	VkSubmitInfo submit = {
		.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO,
		.commandBufferCount = 1,
		.pCommandBuffers = &g_command,
	};
	if (p_vkQueueSubmit(g_queue, 1, &submit, g_fence) != VK_SUCCESS)
		return -1;
	VkFenceGetFdInfoKHR get_fd_info = {
		.sType = VK_STRUCTURE_TYPE_FENCE_GET_FD_INFO_KHR,
		.fence = g_fence,
		.handleType = VK_EXTERNAL_FENCE_HANDLE_TYPE_SYNC_FD_BIT_KHR,
	};
	int fence_fd = -1;
	if (p_vkGetFenceFdKHR(g_device, &get_fd_info, &fence_fd) != VK_SUCCESS ||
	    fence_fd < 0)
		return -1;
	*out_fence_fd = fence_fd;
	g_active_slot = -1;
	return 0;
}

static int import_buffer(const VkPhysicalDeviceMemoryProperties *memory_properties,
			 uint32_t width, uint32_t height, uint32_t pitch,
			 int dma_fd, VkBufferUsageFlags usage,
			 struct imported_buffer *output)
{
	VkDeviceSize size = (VkDeviceSize)pitch * height;
	VkExternalMemoryBufferCreateInfo external = {
		.sType = VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_BUFFER_CREATE_INFO,
		.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT,
	};
	VkBufferCreateInfo buffer_info = {
		.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
		.pNext = &external,
		.size = size,
		.usage = usage,
		.sharingMode = VK_SHARING_MODE_EXCLUSIVE,
	};
	VkResult result = p_vkCreateBuffer(g_device, &buffer_info, NULL,
					   &output->buffer);
	if (result != VK_SUCCESS) {
		fprintf(stderr,
			"saai-gpu-compositor: external buffer create failed: %d\n",
			result);
		return -1;
	}

	VkMemoryRequirements requirements;
	p_vkGetBufferMemoryRequirements(g_device, output->buffer, &requirements);
	VkMemoryFdPropertiesKHR fd_properties = {
		.sType = VK_STRUCTURE_TYPE_MEMORY_FD_PROPERTIES_KHR,
	};
	result = p_vkGetMemoryFdPropertiesKHR(
		g_device, VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT,
		dma_fd, &fd_properties);
	if (result != VK_SUCCESS) {
		fprintf(stderr,
			"saai-gpu-compositor: dma-buf properties failed: %d\n",
			result);
		return -1;
	}
	uint32_t memory_type = find_memory_type(
		memory_properties,
		requirements.memoryTypeBits & fd_properties.memoryTypeBits, 0);
	if (memory_type == UINT32_MAX) {
		fprintf(stderr,
			"saai-gpu-compositor: incompatible memory bits buffer=%x fd=%x\n",
			requirements.memoryTypeBits, fd_properties.memoryTypeBits);
		return -1;
	}
	fprintf(stderr,
		"saai-gpu-compositor: importing %ux%u pitch=%u size=%llu requirement=%llu\n",
		width, height, pitch, (unsigned long long)size,
		(unsigned long long)requirements.size);
	VkMemoryDedicatedAllocateInfo dedicated = {
		.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO,
		.buffer = output->buffer,
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
		fprintf(stderr,
			"saai-gpu-compositor: dma-buf import allocation failed: %d\n",
			result);
		close(import.fd);
		return -1;
	}
	result = p_vkBindBufferMemory(g_device, output->buffer, output->memory, 0);
	if (result != VK_SUCCESS)
		fprintf(stderr,
			"saai-gpu-compositor: imported buffer bind failed: %d\n",
			result);
	return result == VK_SUCCESS ? 0 : -1;
}

static void destroy_imported_buffer(struct imported_buffer *buffer)
{
	if (buffer->buffer != VK_NULL_HANDLE)
		p_vkDestroyBuffer(g_device, buffer->buffer, NULL);
	if (buffer->memory != VK_NULL_HANDLE)
		p_vkFreeMemory(g_device, buffer->memory, NULL);
	memset(buffer, 0, sizeof(*buffer));
}

static int gpu_blit_dmabuf(int dma_fd, uint32_t width, uint32_t height,
			   uint32_t stride, uint32_t output_pitch,
			   uint32_t format)
{
	if (g_active_slot < 0 || width == 0 || height == 0 ||
	    stride < width * 4 ||
	    (format != DRM_FORMAT_XRGB8888 && format != DRM_FORMAT_ARGB8888))
		return -1;
	int contiguous = stride == output_pitch && width * 4 == output_pitch;
	VkBufferCopy *rows = NULL;
	if (!contiguous) {
		rows = calloc(height, sizeof(*rows));
		if (!rows)
			return -1;
		for (uint32_t row = 0; row < height; ++row) {
			rows[row].srcOffset = (VkDeviceSize)row * stride;
			rows[row].dstOffset = (VkDeviceSize)row * output_pitch;
			rows[row].size = (VkDeviceSize)width * 4;
		}
	}

	struct imported_buffer source = {0};
	if (import_buffer(&g_memory_properties, width, height, stride, dma_fd,
			  VK_BUFFER_USAGE_TRANSFER_SRC_BIT, &source) < 0) {
		free(rows);
		destroy_imported_buffer(&source);
		return -1;
	}
	if (g_pending_import_count >= MAX_PENDING_IMPORTS) {
		free(rows);
		destroy_imported_buffer(&source);
		return -1;
	}

	VkBufferMemoryBarrier acquire = buffer_barrier(source.buffer);
	acquire.srcAccessMask = 0;
	acquire.dstAccessMask = VK_ACCESS_TRANSFER_READ_BIT;
	acquire.srcQueueFamilyIndex = VK_QUEUE_FAMILY_FOREIGN_EXT;
	acquire.dstQueueFamilyIndex = g_queue_family;
	p_vkCmdPipelineBarrier(g_command,
		VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,
		VK_PIPELINE_STAGE_TRANSFER_BIT, 0,
		0, NULL, 1, &acquire, 0, NULL);

	if (contiguous) {
		VkBufferCopy copy = {
			.srcOffset = 0,
			.dstOffset = 0,
			.size = (VkDeviceSize)stride * height,
		};
		p_vkCmdCopyBuffer(g_command, source.buffer,
			g_scanout[g_active_slot].buffer, 1, &copy);
	} else {
		p_vkCmdCopyBuffer(g_command, source.buffer,
			g_scanout[g_active_slot].buffer, height, rows);
	}
	free(rows);

	VkBufferMemoryBarrier release = buffer_barrier(source.buffer);
	release.srcAccessMask = VK_ACCESS_TRANSFER_READ_BIT;
	release.dstAccessMask = 0;
	release.srcQueueFamilyIndex = g_queue_family;
	release.dstQueueFamilyIndex = VK_QUEUE_FAMILY_FOREIGN_EXT;
	p_vkCmdPipelineBarrier(g_command,
		VK_PIPELINE_STAGE_TRANSFER_BIT,
		VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, 0,
		0, NULL, 1, &release, 0, NULL);

	/* ADR-024 continued: recorded into the batch already opened by
	 * gpu_begin() -- no submit here. `source` must stay alive until
	 * gpu_end()'s frame-wide submit_commands() actually completes, so
	 * its destruction is deferred there instead of happening right
	 * after this call returns. */
	g_pending_imports[g_pending_import_count++] = source;
	return 0;
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
	LOAD(vkAllocateMemory);
	LOAD(vkFreeMemory);
	LOAD(vkCreateBuffer);
	LOAD(vkDestroyBuffer);
	LOAD(vkGetBufferMemoryRequirements);
	LOAD(vkBindBufferMemory);
	LOAD(vkMapMemory);
	LOAD(vkCreateCommandPool);
	LOAD(vkAllocateCommandBuffers);
	LOAD(vkResetCommandPool);
	LOAD(vkBeginCommandBuffer);
	LOAD(vkCmdPipelineBarrier);
	LOAD(vkCmdFillBuffer);
	LOAD(vkCmdCopyBuffer);
	LOAD(vkEndCommandBuffer);
	LOAD(vkQueueSubmit);
	LOAD(vkQueueWaitIdle);
	LOAD(vkCreateFence);
	LOAD(vkDestroyFence);
	LOAD(vkWaitForFences);
	LOAD(vkResetFences);
	LOAD(vkGetMemoryFdPropertiesKHR);
	LOAD(vkGetFenceFdKHR);

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
		VK_KHR_DEDICATED_ALLOCATION_EXTENSION_NAME,
		VK_KHR_GET_MEMORY_REQUIREMENTS_2_EXTENSION_NAME,
		VK_KHR_BIND_MEMORY_2_EXTENSION_NAME,
		VK_EXT_QUEUE_FAMILY_FOREIGN_EXTENSION_NAME,
		/* ADR-024 continued: full async submit pipeline -- confirmed
		 * present on this UMD via a standalone probe tool
		 * (vk-extcheck.c) before relying on it here, not assumed. */
		VK_KHR_EXTERNAL_FENCE_EXTENSION_NAME,
		VK_KHR_EXTERNAL_FENCE_FD_EXTENSION_NAME,
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

	p_vkGetPhysicalDeviceMemoryProperties(physical, &g_memory_properties);
	if (import_buffer(&g_memory_properties, width, height, pitch, dma_fd0,
			  VK_BUFFER_USAGE_TRANSFER_DST_BIT, &g_scanout[0]) < 0 ||
	    import_buffer(&g_memory_properties, width, height, pitch, dma_fd1,
			  VK_BUFFER_USAGE_TRANSFER_DST_BIT, &g_scanout[1]) < 0)
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
		&g_memory_properties, staging_requirements.memoryTypeBits,
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
	VkExportFenceCreateInfoKHR export_fence_info = {
		.sType = VK_STRUCTURE_TYPE_EXPORT_FENCE_CREATE_INFO_KHR,
		.handleTypes = VK_EXTERNAL_FENCE_HANDLE_TYPE_SYNC_FD_BIT_KHR,
	};
	VkFenceCreateInfo fence_info = {
		.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO,
		.pNext = &export_fence_info,
	};
	if (p_vkCreateFence(g_device, &fence_info, NULL, &g_fence) != VK_SUCCESS)
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
	if (argc != 7) {
		fprintf(stderr,
			"usage: %s WIDTH HEIGHT PITCH DMA_FD0 DMA_FD1 FD_SOCKET\n",
			argv[0]);
		return 64;
	}
	uint32_t width, height, pitch, fd0, fd1, fd_socket;
	if (parse_u32(argv[1], &width) < 0 ||
	    parse_u32(argv[2], &height) < 0 ||
	    parse_u32(argv[3], &pitch) < 0 ||
	    parse_u32(argv[4], &fd0) < 0 ||
	    parse_u32(argv[5], &fd1) < 0 ||
	    parse_u32(argv[6], &fd_socket) < 0 ||
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
					  command.stride, pitch);
			break;
		case OP_END: {
			/* ADR-024 continued: OP_END owns its own full
			 * reply/cleanup cycle instead of falling through to
			 * the shared ack-write below, because the ordering
			 * matters and is easy to get backwards: displayd
			 * must receive the exported fence fd and the 'K' ack
			 * BEFORE this process's own (now-async) wait for
			 * that same fence -- otherwise this would just be
			 * the old per-op-blocking behavior again with extra
			 * steps. The internal wait below, on this process's
			 * own retained copy of the fd, is what makes it safe
			 * to destroy `g_pending_imports` (this frame's
			 * client dma-buf imports) afterward -- displayd has
			 * already moved on by that point and does not wait
			 * for it. */
			int fence_fd = -1;
			if (gpu_end(command.slot, &fence_fd) < 0) {
				fprintf(stderr,
					"saai-gpu-compositor: GPU operation %u failed\n",
					command.op);
				return 76;
			}
			int fence_fd_copy = dup(fence_fd);
			if (fence_fd_copy < 0 ||
			    send_fd((int)fd_socket, fence_fd) < 0) {
				fprintf(stderr,
					"saai-gpu-compositor: fence fd export/send failed\n");
				return 76;
			}
			close(fence_fd);
			if (write_full(STDOUT_FILENO, "K", 1) < 0)
				return 77;
			struct pollfd pfd = {
				.fd = fence_fd_copy,
				.events = POLLIN,
			};
			/* Same 2s bound the old vkWaitForFences call used --
			 * a genuinely wedged GPU still fails loudly instead
			 * of leaking imports forever, it just no longer
			 * blocks displayd while doing so. poll() returning
			 * 0 (timeout) or an error both fall through to the
			 * same cleanup below; if the GPU truly never
			 * finished, whatever destroy_imported_buffer() does
			 * next is no less safe than before batching/async
			 * existed (this file already accepted "wedged GPU"
			 * as a fail-fast-the-whole-process condition, not a
			 * leak-avoidance one). */
			poll(&pfd, 1, 2000);
			close(fence_fd_copy);
			for (int i = 0; i < g_pending_import_count; ++i)
				destroy_imported_buffer(&g_pending_imports[i]);
			g_pending_import_count = 0;
			g_batch_pending_staging_blit = 0;
			continue;
		}
		case OP_DMABUF_BLIT: {
			int dma_fd = receive_fd((int)fd_socket);
			if (dma_fd >= 0 && command.width <= width &&
			    command.height <= height && command.payload_len == 0) {
				result = gpu_blit_dmabuf(
					dma_fd, command.width, command.height,
					command.stride, pitch, command.arg);
				close(dma_fd);
			}
			if (result < 0) {
				fprintf(stderr,
					"saai-gpu-compositor: client dma-buf import/copy rejected\n");
				if (write_full(STDOUT_FILENO, "F", 1) < 0)
					return 77;
				continue;
			}
			break;
		}
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
