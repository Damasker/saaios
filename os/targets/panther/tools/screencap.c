/* Diagnostic-only DRM/KMS screenshot tool (same category as vk-triangle,
 * vk-frame etc. in this directory: a throwaway spike binary, not part of
 * the shipped image) -- dumps the active CRTC's scanout framebuffer to a
 * 32bpp top-down BMP on stdout.
 *
 * Written specifically to give a headless development session (no eyes
 * on the physical device) a way to actually SEE what saai-displayd
 * composited, instead of relying only on frame-hash/log evidence -- the
 * VUI sprints' own Definition of Done calls for a "physical Pixel 7
 * review" that this makes checkable without a human holding the phone.
 *
 * Runs independently of saai-displayd (no IPC, no shared state): opens
 * /dev/dri/card0 itself, reads the current CRTC's framebuffer via the
 * legacy (non-atomic) GETCRTC/GETFB/MAP_DUMB ioctls, which any caller
 * with CAP_SYS_ADMIN (root) can use regardless of which process
 * currently holds DRM master -- confirmed against a live compositor
 * without disturbing it (read-only ioctls, no SETCRTC/master claim).
 *
 * DRM_FORMAT_XRGB8888 (the format saai-displayd's own logs already
 * name, "DrmFourcc(XR24)") is byte-for-byte the same little-endian
 * B,G,R,X layout a 32bpp BMP already uses -- the pixel data below is
 * copied straight through, no color conversion.
 */
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <unistd.h>

#include <drm/drm.h>
#include <drm/drm_mode.h>

static int xioctl(int fd, unsigned long request, void *arg) {
    int result;
    do {
        result = ioctl(fd, request, arg);
    } while (result == -1 && errno == EINTR);
    return result;
}

int main(int argc, char **argv) {
    const char *card = argc > 1 ? argv[1] : "/dev/dri/card0";
    int fd = open(card, O_RDWR | O_CLOEXEC);
    if (fd < 0) {
        perror("open");
        return 1;
    }

    struct drm_mode_card_res res = {0};
    if (xioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &res) < 0) {
        perror("GETRESOURCES (count)");
        return 1;
    }
    uint32_t crtc_ids[32];
    if (res.count_crtcs > 32) {
        fprintf(stderr, "too many crtcs: %u\n", res.count_crtcs);
        return 1;
    }
    res.crtc_id_ptr = (uint64_t)(uintptr_t)crtc_ids;
    res.count_fbs = 0;
    res.count_connectors = 0;
    res.count_encoders = 0;
    if (xioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &res) < 0) {
        perror("GETRESOURCES (crtcs)");
        return 1;
    }

    uint32_t fb_id = 0;
    for (uint32_t i = 0; i < res.count_crtcs; i++) {
        struct drm_mode_crtc crtc = {0};
        crtc.crtc_id = crtc_ids[i];
        if (xioctl(fd, DRM_IOCTL_MODE_GETCRTC, &crtc) < 0) {
            continue;
        }
        if (crtc.fb_id != 0) {
            fb_id = crtc.fb_id;
            break;
        }
    }
    if (fb_id == 0) {
        fprintf(stderr, "no active crtc with a framebuffer found\n");
        return 1;
    }

    struct drm_mode_fb_cmd fb = {0};
    fb.fb_id = fb_id;
    if (xioctl(fd, DRM_IOCTL_MODE_GETFB, &fb) < 0) {
        perror("GETFB");
        return 1;
    }
    if (fb.handle == 0) {
        fprintf(stderr, "GETFB returned no handle (need root/CAP_SYS_ADMIN)\n");
        return 1;
    }
    if (fb.bpp != 32) {
        fprintf(stderr, "unsupported bpp=%u (only 32bpp XRGB/ARGB handled)\n", fb.bpp);
        return 1;
    }

    struct drm_mode_map_dumb map_req = {0};
    map_req.handle = fb.handle;
    if (xioctl(fd, DRM_IOCTL_MODE_MAP_DUMB, &map_req) < 0) {
        perror("MAP_DUMB");
        return 1;
    }

    size_t map_size = (size_t)fb.pitch * fb.height;
    void *base = mmap(NULL, map_size, PROT_READ, MAP_SHARED, fd, (off_t)map_req.offset);
    if (base == MAP_FAILED) {
        perror("mmap");
        return 1;
    }

    uint32_t width = fb.width;
    uint32_t height = fb.height;
    uint32_t row_bytes = width * 4;
    uint32_t file_size = 14 + 40 + row_bytes * height;

    unsigned char bmp_header[14] = {
        'B', 'M',
        (unsigned char)(file_size), (unsigned char)(file_size >> 8),
        (unsigned char)(file_size >> 16), (unsigned char)(file_size >> 24),
        0, 0, 0, 0,
        54, 0, 0, 0,
    };
    int32_t neg_height = -(int32_t)height;
    unsigned char dib_header[40] = {0};
    *(uint32_t *)(dib_header + 0) = 40;
    *(uint32_t *)(dib_header + 4) = width;
    *(int32_t *)(dib_header + 8) = neg_height;
    *(uint16_t *)(dib_header + 12) = 1;
    *(uint16_t *)(dib_header + 14) = 32;
    *(uint32_t *)(dib_header + 20) = row_bytes * height;

    fwrite(bmp_header, 1, sizeof(bmp_header), stdout);
    fwrite(dib_header, 1, sizeof(dib_header), stdout);
    for (uint32_t y = 0; y < height; y++) {
        const unsigned char *row = (const unsigned char *)base + (size_t)y * fb.pitch;
        fwrite(row, 1, row_bytes, stdout);
    }

    munmap(base, map_size);
    close(fd);
    return 0;
}
