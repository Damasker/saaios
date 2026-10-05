/* Read-only device-mapper view of one logical partition inside super.
 * Does not write the partition, does not execute files from it. */
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <unistd.h>

#define DM_NAME_LEN 128
#define DM_UUID_LEN 129
#define DM_MAX_TYPE_NAME 16
#define DM_IOCTL 0xfd
#define DM_VERSION_CMD 0
#define DM_DEV_CREATE_CMD 3
#define DM_DEV_REMOVE_CMD 4
#define DM_DEV_SUSPEND_CMD 6
#define DM_TABLE_LOAD_CMD 9
#define DM_READONLY_FLAG (1u << 0)
#define DM_SUSPEND_FLAG (1u << 1)

struct dm_ioctl {
    uint32_t version[3];
    uint32_t data_size;
    uint32_t data_start;
    uint32_t target_count;
    int32_t open_count;
    uint32_t flags;
    uint32_t event_nr;
    uint32_t padding;
    uint64_t dev;
    char name[DM_NAME_LEN];
    char uuid[DM_UUID_LEN];
    char data[7];
};

struct dm_target_spec {
    uint64_t sector_start;
    uint64_t length;
    int32_t status;
    uint32_t next;
    char target_type[DM_MAX_TYPE_NAME];
};

#define _IOC(dir, type, nr, size) (((dir) << 30) | ((type) << 8) | (nr) | ((size) << 16))
#define _IOWR(type, nr, size) _IOC(3u, (type), (nr), sizeof(size))

struct lp_part {
    char name[36];
    uint32_t attributes;
    uint32_t first_extent;
    uint32_t num_extents;
    uint32_t group_index;
} __attribute__((packed));

struct lp_ext {
    uint64_t num_sectors;
    uint32_t target_type;
    uint64_t target_data;
    uint32_t target_source;
} __attribute__((packed));

_Static_assert(sizeof(struct dm_ioctl) == 312, "dm_ioctl size");
_Static_assert(sizeof(struct dm_target_spec) == 40, "dm_target_spec size");
_Static_assert(sizeof(struct lp_part) == 52, "lp_part size");
_Static_assert(sizeof(struct lp_ext) == 24, "lp_ext size");

static int read_full(int fd, void *buf, size_t n, off_t off) {
    unsigned char *p = buf;
    while (n) {
        ssize_t r = pread(fd, p, n, off);
        if (r < 0) {
            if (errno == EINTR)
                continue;
            return -1;
        }
        if (r == 0)
            return -1;
        p += r;
        n -= (size_t)r;
        off += r;
    }
    return 0;
}

static int dm_fd(void) {
    FILE *f = fopen("/sys/class/misc/device-mapper/dev", "r");
    unsigned major = 0, minor = 0;
    if (!f || fscanf(f, "%u:%u", &major, &minor) != 2) {
        if (f)
            fclose(f);
        fprintf(stderr, "device-mapper control node unknown\n");
        return -1;
    }
    fclose(f);
    unlink("/dev/mapper-control");
    if (mknod("/dev/mapper-control", S_IFCHR | 0600, makedev(major, minor)) < 0)
        return -1;
    return open("/dev/mapper-control", O_RDWR | O_CLOEXEC);
}

static void dm_init(struct dm_ioctl *io, const char *name, uint32_t bytes) {
    memset(io, 0, bytes);
    io->version[0] = 4;
    io->version[1] = 47;
    io->version[2] = 0;
    io->data_size = bytes;
    io->data_start = sizeof(*io);
    snprintf(io->name, sizeof(io->name), "%s", name);
}

static int dm_call(int fd, unsigned cmd, struct dm_ioctl *io) {
    if (ioctl(fd, cmd, io) < 0)
        return -1;
    return 0;
}

static int find_partition(int fd, const char *want, struct lp_ext *out, uint32_t *count) {
    unsigned char geo[64];
    if (read_full(fd, geo, sizeof(geo), 4096) < 0)
        return -1;
    uint32_t magic = 0, max_size = 0, slots = 0;
    memcpy(&magic, geo, 4);
    memcpy(&max_size, geo + 40, 4);
    memcpy(&slots, geo + 44, 4);
    if (magic != 0x616c4467 || max_size < 256 || slots < 1 || slots > 4) {
        fprintf(stderr, "geometry magic=%08x slots=%u max=%u\n", magic, slots, max_size);
        return -1;
    }
    for (uint32_t slot = 0; slot < slots; slot++) {
        off_t base = 12288 + (off_t)slot * max_size;
        unsigned char hdr[128];
        if (read_full(fd, hdr, sizeof(hdr), base) < 0)
            return -1;
        uint32_t hmagic = 0, header_size = 0;
        memcpy(&hmagic, hdr, 4);
        memcpy(&header_size, hdr + 8, 4);
        if (hmagic != 0x414c5030 || header_size < 0x80 || header_size > 4096)
            continue;
        uint32_t part_off = 0, part_num = 0, part_esz = 0;
        uint32_t ext_off = 0, ext_num = 0, ext_esz = 0;
        memcpy(&part_off, hdr + 0x50, 4);
        memcpy(&part_num, hdr + 0x54, 4);
        memcpy(&part_esz, hdr + 0x58, 4);
        memcpy(&ext_off, hdr + 0x5c, 4);
        memcpy(&ext_num, hdr + 0x60, 4);
        memcpy(&ext_esz, hdr + 0x64, 4);
        if (part_esz < sizeof(struct lp_part) || ext_esz < sizeof(struct lp_ext))
            continue;
        if (part_num > 64 || ext_num > 512)
            continue;
        for (uint32_t i = 0; i < part_num; i++) {
            struct lp_part part;
            off_t at = base + header_size + part_off + (off_t)i * part_esz;
            if (read_full(fd, &part, sizeof(part), at) < 0)
                return -1;
            part.name[35] = 0;
            if (strcmp(part.name, want) != 0)
                continue;
            if (part.num_extents == 0 || part.num_extents > 64 ||
                part.first_extent + part.num_extents > ext_num) {
                fprintf(stderr, "%s slot %u bad extents %u+%u of %u\n", want, slot,
                        part.first_extent, part.num_extents, ext_num);
                return -1;
            }
            fprintf(stderr, "found %s in metadata slot %u extents %u..%u\n", want, slot,
                    part.first_extent, part.first_extent + part.num_extents);
            for (uint32_t e = 0; e < part.num_extents; e++) {
                off_t eat = base + header_size + ext_off +
                            (off_t)(part.first_extent + e) * ext_esz;
                if (read_full(fd, &out[e], sizeof(out[e]), eat) < 0)
                    return -1;
                fprintf(stderr, "extent %u type=%u sectors=%llu phys=%llu src=%u\n", e,
                        out[e].target_type, (unsigned long long)out[e].num_sectors,
                        (unsigned long long)out[e].target_data, out[e].target_source);
                if (out[e].target_type != 0 || out[e].num_sectors == 0)
                    return -1;
            }
            *count = part.num_extents;
            return 0;
        }
    }
    fprintf(stderr, "partition %s not found\n", want);
    return -1;
}

static int map_extents(const struct lp_ext *exts, uint32_t count, const char *name) {
    int ctl = dm_fd();
    if (ctl < 0) {
        perror("mapper-control");
        return -1;
    }
    struct dm_ioctl create;
    dm_init(&create, name, sizeof(create));
    unsigned create_cmd = _IOWR(DM_IOCTL, DM_DEV_CREATE_CMD, struct dm_ioctl);
    if (dm_call(ctl, create_cmd, &create) < 0) {
        perror("DM_DEV_CREATE");
        close(ctl);
        return -1;
    }
    size_t cap = sizeof(struct dm_ioctl) + count * 128;
    unsigned char *buf = calloc(1, cap);
    if (!buf) {
        close(ctl);
        return -1;
    }
    struct dm_ioctl *io = (struct dm_ioctl *)buf;
    dm_init(io, name, (uint32_t)cap);
    io->target_count = count;
    io->flags = DM_READONLY_FLAG;
    unsigned char *cursor = buf + sizeof(*io);
    uint64_t logical = 0;
    for (uint32_t i = 0; i < count; i++) {
        struct dm_target_spec *spec = (struct dm_target_spec *)cursor;
        spec->sector_start = logical;
        spec->length = exts[i].num_sectors;
        snprintf(spec->target_type, sizeof(spec->target_type), "linear");
        char *param = (char *)(spec + 1);
        int n = snprintf(param, 64, "259:14 %llu", (unsigned long long)exts[i].target_data);
        if (n < 0 || n >= 64) {
            free(buf);
            close(ctl);
            return -1;
        }
        unsigned step = (unsigned)(sizeof(*spec) + (unsigned)n + 1);
        step = (step + 7u) & ~7u;
        spec->next = step;
        cursor += step;
        logical += exts[i].num_sectors;
    }
    io->data_size = (uint32_t)(cursor - buf);
    unsigned load_cmd = _IOWR(DM_IOCTL, DM_TABLE_LOAD_CMD, struct dm_ioctl);
    if (dm_call(ctl, load_cmd, io) < 0) {
        perror("DM_TABLE_LOAD");
        free(buf);
        close(ctl);
        return -1;
    }
    struct dm_ioctl resume;
    dm_init(&resume, name, sizeof(resume));
    unsigned resume_cmd = _IOWR(DM_IOCTL, DM_DEV_SUSPEND_CMD, struct dm_ioctl);
    if (dm_call(ctl, resume_cmd, &resume) < 0) {
        perror("DM_RESUME");
        free(buf);
        close(ctl);
        return -1;
    }
    unsigned maj = (unsigned)((resume.dev >> 8) & 0xfff);
    unsigned min = (unsigned)((resume.dev & 0xff) | ((resume.dev >> 12) & 0xfff00));
    printf("mapped %s dev=%llu major=%u minor=%u sectors=%llu\n", name,
           (unsigned long long)resume.dev, maj, min, (unsigned long long)logical);
    free(buf);
    close(ctl);
    return 0;
}

static int unmap_name(const char *name) {
    int ctl = dm_fd();
    if (ctl < 0) {
        perror("mapper-control");
        return -1;
    }
    struct dm_ioctl io;
    dm_init(&io, name, sizeof(io));
    unsigned cmd = _IOWR(DM_IOCTL, DM_DEV_REMOVE_CMD, struct dm_ioctl);
    int rc = dm_call(ctl, cmd, &io);
    if (rc < 0)
        perror("DM_DEV_REMOVE");
    close(ctl);
    return rc;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s print|map|unmap NAME\n", argv[0]);
        return 2;
    }
    if (strcmp(argv[1], "unmap") == 0)
        return unmap_name(argv[2]) == 0 ? 0 : 1;
    int fd = open("/dev/super", O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        perror("/dev/super");
        return 1;
    }
    struct lp_ext exts[64];
    uint32_t count = 0;
    if (find_partition(fd, argv[2], exts, &count) < 0) {
        close(fd);
        return 1;
    }
    close(fd);
    if (strcmp(argv[1], "print") == 0)
        return 0;
    if (strcmp(argv[1], "map") == 0)
        return map_extents(exts, count, "vb-ro") == 0 ? 0 : 1;
    fprintf(stderr, "unknown action %s\n", argv[1]);
    return 2;
}
