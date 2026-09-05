#define _GNU_SOURCE

#include <arpa/inet.h>
#include <drm.h>
#include <drm_fourcc.h>
#include <drm_mode.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/input.h>
#include <net/if.h>
#include <poll.h>
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

#define SAAIOS_DRM_MODE_CONNECTED 1

static uint32_t active_connector_id;
static uint32_t active_crtc_id;
static struct drm_mode_modeinfo active_mode;
static int haptic_fd = -1;
static int haptic_effect_id = -1;
static pid_t bluetooth_scan_pid = -1;
static pid_t bluetooth_pair_pid = -1;
static pid_t ai_query_pid = -1;
static int ai_last_action = -1;
static bool bluetooth_saved_view = false;
static int bluetooth_forget_candidate = -1;

static int drm_ioctl(int fd, unsigned long request, void *argument) {
    int result;
    do {
        result = ioctl(fd, request, argument);
    } while (result < 0 && errno == EINTR);
    return result;
}

static uint64_t monotonic_milliseconds(void) {
    struct timespec now = {0};
    if (clock_gettime(CLOCK_MONOTONIC, &now) < 0) {
        return 0;
    }
    return (uint64_t)now.tv_sec * 1000U +
           (uint64_t)now.tv_nsec / 1000000U;
}

static void setup_haptic(void) {
    haptic_fd = open("/dev/input/haptic", O_RDWR | O_CLOEXEC | O_NONBLOCK);
    if (haptic_fd < 0) {
        fprintf(stderr, "drm-splash: haptic unavailable: %s\n",
                strerror(errno));
        return;
    }

    int16_t custom_data[2] = {0, 9};
    struct ff_effect effect = {0};
    effect.type = FF_PERIODIC;
    effect.id = -1;
    effect.direction = 0x0000;
    effect.replay.length = 15;
    effect.u.periodic.waveform = FF_CUSTOM;
    effect.u.periodic.custom_data = custom_data;
    effect.u.periodic.custom_len = 2;
    if (ioctl(haptic_fd, EVIOCSFF, &effect) < 0) {
        fprintf(stderr, "drm-splash: haptic upload failed: %s\n",
                strerror(errno));
        close(haptic_fd);
        haptic_fd = -1;
        return;
    }
    haptic_effect_id = effect.id;
    struct input_event gain = {0};
    gain.type = EV_FF;
    gain.code = FF_GAIN;
    gain.value = 30;
    if (write(haptic_fd, &gain, sizeof(gain)) != (ssize_t)sizeof(gain)) {
        fprintf(stderr, "drm-splash: haptic gain failed: %s\n",
                strerror(errno));
    }
    fprintf(stderr, "drm-splash: haptic ready with effect %d\n",
            haptic_effect_id);
}

static void play_haptic(void) {
    if (haptic_fd < 0 || haptic_effect_id < 0) {
        return;
    }
    struct input_event event = {0};
    event.type = EV_FF;
    event.code = (uint16_t)haptic_effect_id;
    event.value = 1;
    if (write(haptic_fd, &event, sizeof(event)) != (ssize_t)sizeof(event)) {
        fprintf(stderr, "drm-splash: haptic playback failed: %s\n",
                strerror(errno));
    }
}

static void fill_rect(uint32_t *pixels, uint32_t stride_pixels,
                      uint32_t width, uint32_t height,
                      int x, int y, int w, int h, uint32_t color) {
    if (x < 0) { w += x; x = 0; }
    if (y < 0) { h += y; y = 0; }
    if (x + w > (int)width) { w = (int)width - x; }
    if (y + h > (int)height) { h = (int)height - y; }
    if (w <= 0 || h <= 0) { return; }
    for (int row = y; row < y + h; ++row) {
        uint32_t *line = pixels + (size_t)row * stride_pixels + x;
        for (int column = 0; column < w; ++column) {
            line[column] = color;
        }
    }
}

static const char *glyph(char character) {
    switch (character) {
        case 'S': return "11111" "10000" "10000" "11111" "00001" "00001" "11111";
        case 'A': return "01110" "10001" "10001" "11111" "10001" "10001" "10001";
        case 'B': return "11110" "10001" "10001" "11110" "10001" "10001" "11110";
        case 'C': return "01111" "10000" "10000" "10000" "10000" "10000" "01111";
        case 'D': return "11110" "10001" "10001" "10001" "10001" "10001" "11110";
        case 'E': return "11111" "10000" "10000" "11110" "10000" "10000" "11111";
        case 'F': return "11111" "10000" "10000" "11110" "10000" "10000" "10000";
        case 'G': return "01111" "10000" "10000" "10111" "10001" "10001" "01111";
        case 'H': return "10001" "10001" "10001" "11111" "10001" "10001" "10001";
        case 'I': return "11111" "00100" "00100" "00100" "00100" "00100" "11111";
        case 'J': return "00111" "00010" "00010" "00010" "10010" "10010" "01100";
        case 'K': return "10001" "10010" "10100" "11000" "10100" "10010" "10001";
        case 'L': return "10000" "10000" "10000" "10000" "10000" "10000" "11111";
        case 'M': return "10001" "11011" "10101" "10101" "10001" "10001" "10001";
        case 'N': return "10001" "11001" "10101" "10011" "10001" "10001" "10001";
        case 'O': return "01110" "10001" "10001" "10001" "10001" "10001" "01110";
        case 'P': return "11110" "10001" "10001" "11110" "10000" "10000" "10000";
        case 'Q': return "01110" "10001" "10001" "10001" "10101" "10010" "01101";
        case 'R': return "11110" "10001" "10001" "11110" "10100" "10010" "10001";
        case 'T': return "11111" "00100" "00100" "00100" "00100" "00100" "00100";
        case 'U': return "10001" "10001" "10001" "10001" "10001" "10001" "01110";
        case 'V': return "10001" "10001" "10001" "10001" "10001" "01010" "00100";
        case 'W': return "10001" "10001" "10001" "10101" "10101" "11011" "10001";
        case 'X': return "10001" "10001" "01010" "00100" "01010" "10001" "10001";
        case 'Y': return "10001" "10001" "01010" "00100" "00100" "00100" "00100";
        case 'Z': return "11111" "00001" "00010" "00100" "01000" "10000" "11111";
        case '0': return "01110" "10001" "10011" "10101" "11001" "10001" "01110";
        case '1': return "00100" "01100" "00100" "00100" "00100" "00100" "01110";
        case '2': return "01110" "10001" "00001" "00010" "00100" "01000" "11111";
        case '3': return "11110" "00001" "00001" "01110" "00001" "00001" "11110";
        case '4': return "00010" "00110" "01010" "10010" "11111" "00010" "00010";
        case '5': return "11111" "10000" "10000" "11110" "00001" "00001" "11110";
        case '6': return "01110" "10000" "10000" "11110" "10001" "10001" "01110";
        case '7': return "11111" "00001" "00010" "00100" "01000" "01000" "01000";
        case '8': return "01110" "10001" "10001" "01110" "10001" "10001" "01110";
        case '9': return "01110" "10001" "10001" "01111" "00001" "00001" "01110";
        case '.': return "00000" "00000" "00000" "00000" "00000" "00100" "00100";
        case '-': return "00000" "00000" "00000" "11111" "00000" "00000" "00000";
        case ':': return "00000" "00100" "00100" "00000" "00100" "00100" "00000";
        case '%': return "11001" "11010" "00100" "01000" "10110" "00110" "00000";
        default:  return "00000" "00000" "00000" "00000" "00000" "00000" "00000";
    }
}

static void draw_word(uint32_t *pixels, uint32_t stride_pixels,
                      uint32_t width, uint32_t height,
                      const char *word, int scale, int center_y,
                      uint32_t color) {
    size_t length = strlen(word);
    int total_width = ((int)length * 5 + (int)length - 1) * scale;
    int origin_x = ((int)width - total_width) / 2;
    int origin_y = center_y - (7 * scale) / 2;
    for (size_t letter = 0; letter < length; ++letter) {
        const char *bits = glyph(word[letter]);
        int letter_x = origin_x + (int)letter * 6 * scale;
        for (int row = 0; row < 7; ++row) {
            for (int column = 0; column < 5; ++column) {
                if (bits[row * 5 + column] == '1') {
                    fill_rect(pixels, stride_pixels, width, height,
                              letter_x + column * scale,
                              origin_y + row * scale,
                              scale, scale, color);
                }
            }
        }
    }
}

static void render_splash(uint32_t *pixels, uint32_t stride_pixels,
                          uint32_t width, uint32_t height) {
    for (uint32_t y = 0; y < height; ++y) {
        uint32_t blue = 22 + (42 * y) / (height ? height : 1);
        uint32_t green = 7 + (13 * y) / (height ? height : 1);
        uint32_t red = 5 + (8 * y) / (height ? height : 1);
        for (uint32_t x = 0; x < width; ++x) {
            uint32_t glow_x = x > width / 2 ? x - width / 2 : width / 2 - x;
            uint32_t glow_y = y > height / 2 ? y - height / 2 : height / 2 - y;
            uint32_t glow = 0;
            if (glow_x < width / 3 && glow_y < height / 5) {
                glow = 22 - (22 * glow_x) / (width / 3 + 1);
            }
            uint32_t r = red + glow / 4;
            uint32_t g = green + glow / 2;
            uint32_t b = blue + glow;
            pixels[(size_t)y * stride_pixels + x] =
                ((r > 255 ? 255 : r) << 16) |
                ((g > 255 ? 255 : g) << 8) |
                (b > 255 ? 255 : b);
        }
    }

    int scale = (int)width / 42;
    if (scale < 8) { scale = 8; }
    draw_word(pixels, stride_pixels, width, height,
              "SAAIOS", scale, (int)height / 6, 0x00F4F7FF);

    int line_width = (int)width * 3 / 5;
    int line_y = (int)height / 6 + 5 * scale;
    fill_rect(pixels, stride_pixels, width, height,
              ((int)width - line_width) / 2, line_y,
              line_width, scale / 5 + 2, 0x004F8CFF);

    int dot = scale / 2;
    int dot_y = line_y + scale * 2;
    fill_rect(pixels, stride_pixels, width, height,
              (int)width / 2 - dot / 2, dot_y, dot, dot, 0x006EE7FF);
}

static void render_launcher(uint32_t *pixels, uint32_t stride_pixels,
                            uint32_t width, uint32_t height,
                            int selection, bool active) {
    static const char *const labels[] = {
        "STATUS", "NETWORK", "SOUND", "BLUETOOTH", "CONSOLE"
    };
    render_splash(pixels, stride_pixels, width, height);
    draw_word(pixels, stride_pixels, width, height,
              "MENU", 13, 590, 0x008CA9C8);
    for (int index = 0; index < 5; ++index) {
        int top = 680 + index * 275;
        uint32_t color = index == selection
            ? (active ? 0x0000A0C8 : 0x00285F98)
            : 0x00131F35;
        fill_rect(pixels, stride_pixels, width, height,
                  90, top, (int)width - 180, 195, color);
        draw_word(pixels, stride_pixels, width, height,
                  labels[index], 12, top + 98,
                  index == selection ? 0x00FFFFFF : 0x0092A6BE);
    }
    draw_word(pixels, stride_pixels, width, height,
              active ? "READY" : "TOUCH", 11, 2150,
              active ? 0x0000E7FF : 0x005B789A);
}

static void uppercase_label(char *text) {
    for (size_t index = 0; text[index]; ++index) {
        unsigned char character = (unsigned char)text[index];
        if (character >= 'a' && character <= 'z') {
            text[index] = (char)(character - 'a' + 'A');
        } else if (!((character >= 'A' && character <= 'Z') ||
                     (character >= '0' && character <= '9') ||
                     character == '-' || character == '.')) {
            text[index] = '-';
        }
    }
}

static int read_network_names(char names[][32], int maximum) {
    FILE *file = fopen("/run/wifi-scan.log", "r");
    if (!file) {
        return 0;
    }
    int count = 0;
    char line[256];
    while (count < maximum && fgets(line, sizeof(line), file)) {
        char *separator = strchr(line, '\t');
        if (!separator) {
            continue;
        }
        *separator = '\0';
        snprintf(names[count], 32, "%s", line);
        uppercase_label(names[count]);
        ++count;
    }
    fclose(file);
    return count;
}

static bool read_wifi_address(char *address, size_t address_size) {
    int fd = socket(AF_INET, SOCK_DGRAM | SOCK_CLOEXEC, 0);
    if (fd < 0) {
        return false;
    }
    struct ifreq request = {0};
    snprintf(request.ifr_name, sizeof(request.ifr_name), "wlan0");
    bool ready = false;
    if (ioctl(fd, SIOCGIFADDR, &request) == 0) {
        struct sockaddr_in *socket_address =
            (struct sockaddr_in *)&request.ifr_addr;
        ready = inet_ntop(AF_INET, &socket_address->sin_addr,
                          address, address_size) != NULL;
    }
    close(fd);
    return ready;
}

static bool read_first_line(const char *path, char *value, size_t value_size) {
    FILE *file = fopen(path, "r");
    if (!file) {
        return false;
    }
    bool ready = fgets(value, (int)value_size, file) != NULL;
    fclose(file);
    if (!ready) {
        return false;
    }
    value[strcspn(value, "\r\n")] = '\0';
    return value[0] != '\0';
}

#define AI_LINE_CHARS 26

static bool ai_query_running(void) {
    if (ai_query_pid <= 0) {
        return false;
    }
    if (kill(ai_query_pid, 0) == 0 || errno == EPERM) {
        return true;
    }
    ai_query_pid = -1;
    return false;
}

static int read_ai_lines(char lines[][AI_LINE_CHARS + 1], int maximum) {
    FILE *file = fopen("/run/saaios-ai-ui.log", "r");
    if (!file) {
        return 0;
    }
    char flat[768] = {0};
    size_t length = 0;
    bool pending_space = false;
    int value;
    while (length + 1 < sizeof(flat) && (value = fgetc(file)) != EOF) {
        unsigned char character = (unsigned char)value;
        char mapped = '\0';
        if (character >= 'a' && character <= 'z') {
            mapped = (char)(character - 'a' + 'A');
        } else if ((character >= 'A' && character <= 'Z') ||
                   (character >= '0' && character <= '9') ||
                   character == '.' || character == '-' ||
                   character == ':' || character == '%') {
            mapped = (char)character;
        } else {
            pending_space = length > 0;
        }
        if (mapped) {
            if (pending_space && length + 1 < sizeof(flat) &&
                flat[length - 1] != ' ') {
                flat[length++] = ' ';
            }
            pending_space = false;
            flat[length++] = mapped;
        }
    }
    fclose(file);
    flat[length] = '\0';
    if (length == 0) {
        return 0;
    }

    int count = 0;
    char *save = NULL;
    for (char *word = strtok_r(flat, " ", &save);
         word && count < maximum;
         word = strtok_r(NULL, " ", &save)) {
        size_t word_length = strlen(word);
        if (word_length > AI_LINE_CHARS) {
            word[AI_LINE_CHARS] = '\0';
            word_length = AI_LINE_CHARS;
        }
        size_t line_length = count > 0 ? strlen(lines[count - 1]) : 0;
        if (count == 0 || line_length + 1 + word_length > AI_LINE_CHARS) {
            snprintf(lines[count], AI_LINE_CHARS + 1, "%s", word);
            ++count;
        } else {
            strncat(lines[count - 1], " ", AI_LINE_CHARS - line_length);
            line_length = strlen(lines[count - 1]);
            strncat(lines[count - 1], word, AI_LINE_CHARS - line_length);
        }
    }
    return count;
}

static void start_ai_query(int selected) {
    static const char *const prompts[] = {
        "Use system.metrics exactly once. Reply with at most three short lines in uppercase ASCII English. Summarize CPU, free memory, and load. Do not call any other tool.",
        "Use network.status exactly once. Reply with at most three short lines in uppercase ASCII English. Summarize interfaces and connectivity. Do not call any other tool.",
        "Use system.disk exactly once. Reply with at most three short lines in uppercase ASCII English. Summarize total and free storage. Do not call any other tool."
    };
    if (selected < 0 || selected >= (int)(sizeof(prompts) / sizeof(prompts[0])) ||
        ai_query_running()) {
        return;
    }
    (void)unlink("/run/saaios-ai-ui.log");
    ai_last_action = selected;
    pid_t child = fork();
    if (child == 0) {
        int log = open("/run/saaios-ai-ui.log",
                       O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
        if (log >= 0) {
            (void)dup2(log, STDOUT_FILENO);
            (void)dup2(log, STDERR_FILENO);
            close(log);
        }
        execl("/saaios/saaios-console", "saaios-console",
              "--ask", prompts[selected], NULL);
        dprintf(STDOUT_FILENO, "MODEL OFFLINE\n");
        _exit(127);
    }
    if (child > 0) {
        ai_query_pid = child;
    }
}

static void render_networks(uint32_t *pixels, uint32_t stride_pixels,
                            uint32_t width, uint32_t height) {
    char names[3][32] = {{0}};
    char address[INET_ADDRSTRLEN] = {0};
    int count = read_network_names(names, 3);
    bool online = read_wifi_address(address, sizeof(address));
    render_splash(pixels, stride_pixels, width, height);
    draw_word(pixels, stride_pixels, width, height,
              "NETWORKS", 12, 560, 0x00FFFFFF);
    draw_word(pixels, stride_pixels, width, height,
              online ? "ONLINE" : "WIFI READY", 10, 715,
              online ? 0x0000E7A8 : 0x008CA9C8);
    if (online) {
        draw_word(pixels, stride_pixels, width, height,
                  address, 9, 840, 0x00B7CBE2);
    }
    if (count == 0) {
        draw_word(pixels, stride_pixels, width, height,
                  "SCANNING", 11, 1260, 0x008CA9C8);
    } else {
        for (int index = 0; index < count; ++index) {
            int top = 980 + index * 285;
            fill_rect(pixels, stride_pixels, width, height,
                      70, top, (int)width - 140, 200, 0x001D426B);
            int length = (int)strlen(names[index]);
            int scale = length > 13 ? 8 : (length > 9 ? 10 : 12);
            draw_word(pixels, stride_pixels, width, height,
                      names[index], scale, top + 100, 0x00FFFFFF);
        }
    }
    draw_word(pixels, stride_pixels, width, height,
              "TOUCH TO GO BACK", 7, 2040, 0x005B789A);
}

static int read_bluetooth_names(char names[][32], int maximum,
                                bool *scanning, bool *finished,
                                bool *failed) {
    *scanning = false;
    *finished = false;
    *failed = false;
    FILE *file = fopen("/run/bluetooth-scan.log", "r");
    if (!file) {
        return 0;
    }
    int count = 0;
    char line[256];
    while (fgets(line, sizeof(line), file)) {
        line[strcspn(line, "\r\n")] = '\0';
        if (!strcmp(line, "SCANNING")) {
            *scanning = true;
        } else if (!strncmp(line, "DONE\t", 5)) {
            *finished = true;
            *scanning = false;
        } else if (!strncmp(line, "DEVICE\t", 7) && count < maximum) {
            snprintf(names[count], 32, "%s", line + 7);
            uppercase_label(names[count]);
            ++count;
        } else if (!strncmp(line, "TRANSPORT\t", 10)) {
            /* Informational line for diagnostics; device name is above. */
            continue;
        } else if (line[0] != '\0') {
            *failed = true;
            *scanning = false;
        }
    }
    fclose(file);
    return count;
}

static int read_saved_bluetooth_names(char names[][32], int maximum) {
    FILE *file = fopen("/run/bluetooth-saved.log", "r");
    if (!file) return 0;
    int count = 0;
    char line[256];
    while (count < maximum && fgets(line, sizeof(line), file)) {
        line[strcspn(line, "\r\n")] = '\0';
        if (strncmp(line, "SAVED\t", 6)) continue;
        snprintf(names[count], 32, "%s", line + 6);
        uppercase_label(names[count]);
        ++count;
    }
    fclose(file);
    return count;
}

static int read_bluetooth_pair_state(char name[32]) {
    FILE *file = fopen("/run/bluetooth-pair.log", "r");
    if (!file) {
        return 0;
    }
    int state = 0;
    char line[256];
    while (fgets(line, sizeof(line), file)) {
        line[strcspn(line, "\r\n")] = '\0';
        if (!strcmp(line, "PAIRING")) {
            state = 1;
        } else if (!strncmp(line, "PAIRED\t", 7)) {
            snprintf(name, 32, "%s", line + 7);
            uppercase_label(name);
            state = 2;
        } else if (!strncmp(line, "PAIR-ERROR\t", 11)) {
            snprintf(name, 32, "%s", line + 11);
            uppercase_label(name);
            state = -1;
        } else if (!strncmp(line, "FORGOTTEN\t", 10)) {
            snprintf(name, 32, "%s", line + 10);
            uppercase_label(name);
            state = 3;
        } else if (!strncmp(line, "FORGET-ERROR\t", 13)) {
            snprintf(name, 32, "%s", line + 13);
            uppercase_label(name);
            state = -2;
        }
    }
    fclose(file);
    return state;
}

static void render_bluetooth(uint32_t *pixels, uint32_t stride_pixels,
                             uint32_t width, uint32_t height) {
    char names[4][32] = {{0}};
    bool scanning = false;
    bool finished = false;
    bool failed = false;
    int count = bluetooth_saved_view
        ? read_saved_bluetooth_names(names, 4)
        : read_bluetooth_names(names, 4, &scanning, &finished, &failed);
    char pair_name[32] = {0};
    int pair_state = read_bluetooth_pair_state(pair_name);
    render_splash(pixels, stride_pixels, width, height);
    draw_word(pixels, stride_pixels, width, height,
              "BLUETOOTH", 12, 545, 0x00FFFFFF);
    const char *status = NULL;
    uint32_t status_color = 0x0000E7A8;
    if (bluetooth_saved_view) {
        if (bluetooth_forget_candidate >= 0) {
            status = "TOUCH AGAIN TO FORGET";
            status_color = 0x00E78A68;
        } else if (pair_state == 3) {
            status = "DEVICE FORGOTTEN";
        } else if (pair_state == -2) {
            status = "FORGET FAILED";
            status_color = 0x00E78A68;
        } else {
            status = "SAVED DEVICES";
        }
    } else {
        status = pair_state == 1 ? "PAIRING" :
            (pair_state == 2 ? "PAIRED" :
            (pair_state < 0 ? "PAIR FAILED" :
            (failed ? "SCAN ERROR" :
            (scanning ? "SCANNING" :
            (finished && count > 0 ? "TOUCH A DEVICE" :
             "SCAN COMPLETE")))));
        if (failed || pair_state < 0) status_color = 0x00E78A68;
        else if (scanning || pair_state == 1) status_color = 0x0000E7FF;
    }
    draw_word(pixels, stride_pixels, width, height,
              status, 8, 710, status_color);
    if (count == 0) {
        draw_word(pixels, stride_pixels, width, height,
                  bluetooth_saved_view ? "NO SAVED DEVICES" :
                  (scanning ? "LOOKING FOR DEVICES" :
                  (finished ? "NO NAMED DEVICES" : "BLUETOOTH READY")),
                  8, 1240, 0x008CA9C8);
    } else {
        for (int index = 0; index < count; ++index) {
            int top = 850 + index * 270;
            uint32_t card_color = 0x001D426B;
            if (bluetooth_saved_view &&
                bluetooth_forget_candidate == index) {
                card_color = 0x00804343;
            } else if (!bluetooth_saved_view && pair_state == 2 &&
                       !strcmp(pair_name, names[index])) {
                card_color = 0x00006F5A;
            }
            fill_rect(pixels, stride_pixels, width, height,
                      70, top, (int)width - 140, 190, card_color);
            int length = (int)strlen(names[index]);
            int scale = length > 15 ? 7 : (length > 11 ? 9 : 11);
            draw_word(pixels, stride_pixels, width, height,
                      names[index], scale, top + 95, 0x00FFFFFF);
        }
    }
    draw_word(pixels, stride_pixels, width, height,
              bluetooth_saved_view ? "VOLUME FOR NEARBY" :
                                     "VOLUME FOR SAVED",
              7, 1960, 0x005B789A);
    draw_word(pixels, stride_pixels, width, height,
              "TOUCH EMPTY TO BACK", 7, 2080, 0x005B789A);
}

static int bluetooth_item_at(int y) {
    char names[4][32] = {{0}};
    int count = 0;
    if (bluetooth_saved_view) {
        count = read_saved_bluetooth_names(names, 4);
    } else {
        bool scanning = false;
        bool finished = false;
        bool failed = false;
        count = read_bluetooth_names(names, 4,
                                     &scanning, &finished, &failed);
        if (!finished || scanning || failed) return -1;
    }
    for (int index = 0; index < count; ++index) {
        int top = 850 + index * 270;
        if (y >= top && y < top + 190) {
            return index;
        }
    }
    return -1;
}

static void render_status(uint32_t *pixels, uint32_t stride_pixels,
                          uint32_t width, uint32_t height) {
    char address[INET_ADDRSTRLEN] = {0};
    char capacity[8] = {0};
    char battery[24] = "BATTERY READY";
    char charge_state[24] = "POWER UNKNOWN";
    char utc_time[32] = "TIME SYNCING";
    char raw_brightness[16] = {0};
    char brightness[24] = "BRIGHTNESS 25";
    bool online = read_wifi_address(address, sizeof(address));
    time_t now = time(NULL);
    if (now > 1700000000) {
        struct tm utc = {0};
        if (gmtime_r(&now, &utc)) {
            (void)strftime(utc_time, sizeof(utc_time),
                           "UTC %Y-%m-%d %H-%M", &utc);
        }
    }
    if (read_first_line("/sys/class/power_supply/maxfg/capacity",
                        capacity, sizeof(capacity))) {
        snprintf(battery, sizeof(battery), "BATTERY %.3s", capacity);
    }
    if (read_first_line("/sys/class/power_supply/maxfg/status",
                        charge_state, sizeof(charge_state))) {
        uppercase_label(charge_state);
    }
    if (read_first_line(
            "/sys/devices/platform/1c2c0000.drmdsim/"
            "1c2c0000.drmdsim.0/backlight/panel0-backlight/brightness",
            raw_brightness, sizeof(raw_brightness))) {
        int percent = atoi(raw_brightness) * 100 / 4095;
        if (percent < 0) { percent = 0; }
        if (percent > 100) { percent = 100; }
        snprintf(brightness, sizeof(brightness),
                 "BRIGHTNESS %d", percent);
    }
    render_splash(pixels, stride_pixels, width, height);
    draw_word(pixels, stride_pixels, width, height,
              "STATUS", 13, 540, 0x00FFFFFF);
    draw_word(pixels, stride_pixels, width, height,
              utc_time, 8, 680, 0x00B7CBE2);
    draw_word(pixels, stride_pixels, width, height,
              battery, 10, 810, 0x00B7CBE2);
    draw_word(pixels, stride_pixels, width, height,
              charge_state, 8, 925,
              (!strcmp(charge_state, "CHARGING") ||
               !strcmp(charge_state, "FULL"))
                  ? 0x0000E7A8 : 0x00B7CBE2);
    draw_word(pixels, stride_pixels, width, height,
              brightness, 9, 1040, 0x00FFFFFF);
    draw_word(pixels, stride_pixels, width, height,
              "SYSTEM READY", 10, 1170, 0x0000E7A8);
    draw_word(pixels, stride_pixels, width, height,
              access("/data/saaios/.layout", R_OK) == 0
                  ? "DATA READY" : "DATA OFFLINE",
              10, 1300,
              access("/data/saaios/.layout", R_OK) == 0
                  ? 0x0000E7A8 : 0x00E78A68);
    draw_word(pixels, stride_pixels, width, height,
              "TOUCH READY", 10, 1430, 0x0000E7A8);
    draw_word(pixels, stride_pixels, width, height,
              access("/run/audio-ready", R_OK) == 0
                  ? "AUDIO READY" : "AUDIO STARTING",
              10, 1560,
              access("/run/audio-ready", R_OK) == 0
                  ? 0x0000E7A8 : 0x008CA9C8);
    draw_word(pixels, stride_pixels, width, height,
              online ? "WIFI ONLINE" : "WIFI READY", 10, 1690,
              online ? 0x0000E7A8 : 0x008CA9C8);
    if (online) {
        draw_word(pixels, stride_pixels, width, height,
                  address, 9, 1820, 0x00B7CBE2);
    }
    bool bluetooth_ready =
        access("/sys/class/bluetooth/hci0", R_OK) == 0;
    draw_word(pixels, stride_pixels, width, height,
              bluetooth_ready ? "BLUETOOTH READY" : "BLUETOOTH STARTING",
              8, 1950,
              bluetooth_ready ? 0x0000E7A8 : 0x008CA9C8);
    draw_word(pixels, stride_pixels, width, height,
              "TOUCH TO GO BACK", 7, 2090, 0x005B789A);
}

static void render_console(uint32_t *pixels, uint32_t stride_pixels,
                           uint32_t width, uint32_t height) {
    static const char *const labels[] = {
        "SYSTEM HEALTH", "NETWORK CHECK", "STORAGE CHECK"
    };
    bool running = ai_query_running();
    char lines[4][AI_LINE_CHARS + 1] = {{0}};
    int line_count = running ? 0 : read_ai_lines(lines, 4);
    render_splash(pixels, stride_pixels, width, height);
    draw_word(pixels, stride_pixels, width, height,
              "AI CONSOLE", 11, 430, 0x00FFFFFF);
    draw_word(pixels, stride_pixels, width, height,
              access("/tmp/saaios.sock", R_OK) == 0
                  ? "LOCAL AI READY" : "RUNTIME OFFLINE",
              8, 565,
              access("/tmp/saaios.sock", R_OK) == 0
                  ? 0x0000E7A8 : 0x00E78A68);
    for (int index = 0; index < 3; ++index) {
        int top = 680 + index * 260;
        uint32_t color = index == ai_last_action
            ? 0x00285F98 : 0x001D426B;
        fill_rect(pixels, stride_pixels, width, height,
                  70, top, (int)width - 140, 185, color);
        draw_word(pixels, stride_pixels, width, height,
                  labels[index], 9, top + 93, 0x00FFFFFF);
    }
    if (running) {
        draw_word(pixels, stride_pixels, width, height,
                  "AI THINKING", 10, 1560, 0x0000E7FF);
    } else if (line_count > 0) {
        for (int index = 0; index < line_count; ++index) {
            int scale = strlen(lines[index]) > 22 ? 6 : 7;
            draw_word(pixels, stride_pixels, width, height,
                      lines[index], scale, 1510 + index * 125,
                      0x00B7CBE2);
        }
    } else {
        draw_word(pixels, stride_pixels, width, height,
                  "TOUCH A CHECK", 9, 1620, 0x008CA9C8);
    }
    draw_word(pixels, stride_pixels, width, height,
              "TOUCH EMPTY TO BACK", 7, 2110, 0x005B789A);
}

static void render_sound(uint32_t *pixels, uint32_t stride_pixels,
                         uint32_t width, uint32_t height) {
    bool ready = access("/run/audio-ready", R_OK) == 0;
    bool playing = access("/run/audio-playing", R_OK) == 0;
    char raw_volume[16] = {0};
    char volume_label[24] = "VOLUME 50";
    if (read_first_line("/run/audio-volume",
                        raw_volume, sizeof(raw_volume))) {
        int value = atoi(raw_volume);
        int percent = (value - 400) * 100 / 417;
        if (percent < 0) { percent = 0; }
        if (percent > 100) { percent = 100; }
        snprintf(volume_label, sizeof(volume_label),
                 "VOLUME %d", percent);
    }
    render_splash(pixels, stride_pixels, width, height);
    draw_word(pixels, stride_pixels, width, height,
              "SOUND", 13, 700, 0x00FFFFFF);
    draw_word(pixels, stride_pixels, width, height,
              ready ? "AUDIO READY" : "AUDIO STARTING", 10, 1060,
              ready ? 0x0000E7A8 : 0x008CA9C8);
    draw_word(pixels, stride_pixels, width, height,
              playing ? "PLAYING" : "TEST TONE", 12, 1330,
              playing ? 0x0000E7FF : 0x00B7CBE2);
    draw_word(pixels, stride_pixels, width, height,
              volume_label, 11, 1570, 0x00FFFFFF);
    draw_word(pixels, stride_pixels, width, height,
              "USE VOLUME KEYS", 8, 1740, 0x005B789A);
    draw_word(pixels, stride_pixels, width, height,
              "TOUCH TO GO BACK", 7, 2040, 0x005B789A);
}

static void start_bluetooth_scan(void) {
    if (access("/sys/class/bluetooth/hci0", R_OK) != 0) {
        return;
    }
    if (bluetooth_pair_pid > 0 && kill(bluetooth_pair_pid, 0) == 0) {
        return;
    }
    if (bluetooth_scan_pid > 0 &&
        kill(bluetooth_scan_pid, 0) == 0) {
        return;
    }
    (void)unlink("/run/bluetooth-pair.log");
    pid_t child = fork();
    if (child == 0) {
        int log = open("/run/bluetooth-scan.log",
                       O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
        if (log >= 0) {
            (void)dup2(log, STDOUT_FILENO);
            (void)dup2(log, STDERR_FILENO);
            close(log);
        }
        execl("/saaios/bt-scan", "bt-scan", NULL);
        _exit(127);
    }
    if (child > 0) {
        bluetooth_scan_pid = child;
    }
}

static void start_bluetooth_pair(int selected) {
    if (selected < 0 || selected > 7) {
        return;
    }
    if (bluetooth_pair_pid > 0 &&
        kill(bluetooth_pair_pid, 0) == 0) {
        return;
    }
    pid_t child = fork();
    if (child == 0) {
        int log = open("/run/bluetooth-pair.log",
                       O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
        if (log >= 0) {
            (void)dup2(log, STDOUT_FILENO);
            (void)dup2(log, STDERR_FILENO);
            close(log);
        }
        char item[4];
        snprintf(item, sizeof(item), "%d", selected);
        execl("/saaios/bt-pair", "bt-pair", item, NULL);
        _exit(127);
    }
    if (child > 0) {
        bluetooth_pair_pid = child;
    }
}

static void start_bluetooth_forget(int selected) {
    if (selected < 0 || selected > 7) return;
    if (bluetooth_pair_pid > 0 &&
        kill(bluetooth_pair_pid, 0) == 0) return;
    pid_t child = fork();
    if (child == 0) {
        int log = open("/run/bluetooth-pair.log",
                       O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
        if (log >= 0) {
            (void)dup2(log, STDOUT_FILENO);
            (void)dup2(log, STDERR_FILENO);
            close(log);
        }
        char item[4];
        snprintf(item, sizeof(item), "%d", selected);
        execl("/saaios/bt-pair", "bt-pair", "--forget", item, NULL);
        _exit(127);
    }
    if (child > 0) bluetooth_pair_pid = child;
}

static void start_audio_test(void) {
    if (access("/run/audio-ready", R_OK) != 0) {
        return;
    }
    pid_t child = fork();
    if (child == 0) {
        execl("/saaios/audio-test.sh", "audio-test.sh", NULL);
        _exit(127);
    }
}

static void adjust_audio_volume(bool louder) {
    if (access("/run/audio-ready", R_OK) != 0) {
        return;
    }
    pid_t child = fork();
    if (child == 0) {
        execl("/saaios/audio-volume.sh", "audio-volume.sh",
              louder ? "up" : "down", NULL);
        _exit(127);
    }
}

static void adjust_display_brightness(bool brighter) {
    pid_t child = fork();
    if (child == 0) {
        execl("/saaios/display-brightness.sh", "display-brightness.sh",
              brighter ? "up" : "down", NULL);
        _exit(127);
    }
}

static void render_page(uint32_t *pixels, uint32_t stride_pixels,
                        uint32_t width, uint32_t height, int page,
                        int selection, bool active) {
    if (page == 1) {
        render_status(pixels, stride_pixels, width, height);
    } else if (page == 2) {
        render_networks(pixels, stride_pixels, width, height);
    } else if (page == 3) {
        render_sound(pixels, stride_pixels, width, height);
    } else if (page == 4) {
        render_bluetooth(pixels, stride_pixels, width, height);
    } else if (page == 5) {
        render_console(pixels, stride_pixels, width, height);
    } else {
        render_launcher(pixels, stride_pixels, width, height,
                        selection, active);
    }
}

static void draw_touch_marker(uint32_t *pixels, uint32_t stride_pixels,
                              uint32_t width, uint32_t height, int x, int y) {
    const int outer = 54;
    const int inner = 22;
    fill_rect(pixels, stride_pixels, width, height,
              x - outer / 2, y - outer / 2, outer, outer, 0x0000E7FF);
    fill_rect(pixels, stride_pixels, width, height,
              x - inner / 2, y - inner / 2, inner, inner, 0x00FFFFFF);
}

static void publish_frame(int drm_fd, uint32_t framebuffer_id,
                          uint32_t *pixels, size_t size) {
    (void)msync(pixels, size, MS_SYNC);
    struct drm_mode_fb_dirty_cmd dirty = {0};
    dirty.fb_id = framebuffer_id;
    (void)drm_ioctl(drm_fd, DRM_IOCTL_MODE_DIRTYFB, &dirty);
    struct drm_mode_crtc refresh = {0};
    refresh.set_connectors_ptr = (uintptr_t)&active_connector_id;
    refresh.count_connectors = 1;
    refresh.crtc_id = active_crtc_id;
    refresh.fb_id = framebuffer_id;
    refresh.mode_valid = 1;
    refresh.mode = active_mode;
    if (drm_ioctl(drm_fd, DRM_IOCTL_MODE_SETCRTC, &refresh) < 0) {
        fprintf(stderr, "drm-splash: refresh failed: %s\n", strerror(errno));
    }
}

static void disable_display(int drm_fd) {
    struct drm_mode_crtc request = {0};
    request.crtc_id = active_crtc_id;
    if (drm_ioctl(drm_fd, DRM_IOCTL_MODE_SETCRTC, &request) < 0) {
        fprintf(stderr, "drm-splash: display off failed: %s\n",
                strerror(errno));
    } else {
        fprintf(stderr, "drm-splash: display off\n");
    }
}

static int menu_item_at(int y) {
    for (int index = 0; index < 5; ++index) {
        int top = 680 + index * 275;
        if (y >= top && y < top + 195) {
            return index;
        }
    }
    return -1;
}

static int console_item_at(int y) {
    for (int index = 0; index < 3; ++index) {
        int top = 680 + index * 260;
        if (y >= top && y < top + 185) {
            return index;
        }
    }
    return -1;
}

int main(void) {
    (void)signal(SIGCHLD, SIG_IGN);
    int fd = open("/dev/dri/card0", O_RDWR | O_CLOEXEC);
    if (fd < 0) {
        fprintf(stderr, "drm-splash: open card0 failed: %s\n", strerror(errno));
        return 1;
    }

    struct drm_mode_card_res resources = {0};
    if (drm_ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &resources) < 0) {
        fprintf(stderr, "drm-splash: GETRESOURCES failed: %s\n", strerror(errno));
        return 2;
    }
    uint32_t *crtcs = calloc(resources.count_crtcs, sizeof(*crtcs));
    uint32_t *connectors = calloc(resources.count_connectors, sizeof(*connectors));
    uint32_t *encoders = calloc(resources.count_encoders, sizeof(*encoders));
    resources.crtc_id_ptr = (uintptr_t)crtcs;
    resources.connector_id_ptr = (uintptr_t)connectors;
    resources.encoder_id_ptr = (uintptr_t)encoders;
    if (drm_ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &resources) < 0) {
        fprintf(stderr, "drm-splash: resource list failed: %s\n", strerror(errno));
        return 3;
    }
    fprintf(stderr, "drm-splash: crtcs=%u connectors=%u encoders=%u\n",
            resources.count_crtcs, resources.count_connectors,
            resources.count_encoders);

    struct drm_mode_get_connector selected = {0};
    struct drm_mode_modeinfo *modes = NULL;
    uint32_t *connector_encoders = NULL;
    uint32_t *properties = NULL;
    uint64_t *property_values = NULL;
    uint32_t selected_connector = 0;
    for (uint32_t index = 0; index < resources.count_connectors; ++index) {
        struct drm_mode_get_connector connector = {0};
        connector.connector_id = connectors[index];
        if (drm_ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR, &connector) < 0) {
            continue;
        }
        fprintf(stderr,
                "drm-splash: connector=%u type=%u state=%u modes=%u encoders=%u\n",
                connector.connector_id, connector.connector_type,
                connector.connection, connector.count_modes,
                connector.count_encoders);
        if (connector.connector_type != DRM_MODE_CONNECTOR_DSI ||
            connector.connection != SAAIOS_DRM_MODE_CONNECTED ||
            connector.count_modes == 0) {
            continue;
        }
        modes = calloc(connector.count_modes, sizeof(*modes));
        connector_encoders = calloc(connector.count_encoders,
                                    sizeof(*connector_encoders));
        properties = calloc(connector.count_props, sizeof(*properties));
        property_values = calloc(connector.count_props,
                                 sizeof(*property_values));
        connector.modes_ptr = (uintptr_t)modes;
        connector.encoders_ptr = (uintptr_t)connector_encoders;
        connector.props_ptr = (uintptr_t)properties;
        connector.prop_values_ptr = (uintptr_t)property_values;
        if (drm_ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR, &connector) < 0) {
            fprintf(stderr, "drm-splash: connector detail failed: %s\n",
                    strerror(errno));
            free(modes);
            free(connector_encoders);
            free(properties);
            free(property_values);
            modes = NULL;
            connector_encoders = NULL;
            properties = NULL;
            property_values = NULL;
            continue;
        }
        if (connector.count_modes == 0) {
            fprintf(stderr, "drm-splash: connector detail returned no modes\n");
            free(modes);
            free(connector_encoders);
            free(properties);
            free(property_values);
            modes = NULL;
            connector_encoders = NULL;
            properties = NULL;
            property_values = NULL;
            continue;
        }
        selected = connector;
        selected_connector = connector.connector_id;
        break;
    }
    if (!selected_connector) {
        fprintf(stderr, "drm-splash: no connected DSI mode found\n");
        return 4;
    }

    uint32_t mode_index = 0;
    for (uint32_t index = 0; index < selected.count_modes; ++index) {
        if (modes[index].type & DRM_MODE_TYPE_PREFERRED) {
            mode_index = index;
            break;
        }
    }
    struct drm_mode_modeinfo mode = modes[mode_index];
    fprintf(stderr, "drm-splash: mode=%s %ux%u@%u\n",
            mode.name, mode.hdisplay, mode.vdisplay, mode.vrefresh);

    uint32_t crtc_id = 0;
    uint32_t encoder_count = selected.count_encoders;
    for (uint32_t index = 0; index < encoder_count && !crtc_id; ++index) {
        struct drm_mode_get_encoder encoder = {0};
        encoder.encoder_id = connector_encoders[index];
        if (drm_ioctl(fd, DRM_IOCTL_MODE_GETENCODER, &encoder) < 0) {
            continue;
        }
        if (encoder.crtc_id) {
            crtc_id = encoder.crtc_id;
            break;
        }
        for (uint32_t crtc = 0; crtc < resources.count_crtcs; ++crtc) {
            if (encoder.possible_crtcs & (1U << crtc)) {
                crtc_id = crtcs[crtc];
                break;
            }
        }
    }
    if (!crtc_id && resources.count_crtcs) {
        crtc_id = crtcs[0];
    }
    if (!crtc_id) {
        fprintf(stderr, "drm-splash: no usable CRTC found\n");
        return 5;
    }

    struct drm_mode_create_dumb create = {0};
    create.width = mode.hdisplay;
    create.height = mode.vdisplay;
    create.bpp = 32;
    if (drm_ioctl(fd, DRM_IOCTL_MODE_CREATE_DUMB, &create) < 0) {
        fprintf(stderr, "drm-splash: CREATE_DUMB failed: %s\n", strerror(errno));
        return 6;
    }

    struct drm_mode_fb_cmd2 framebuffer = {0};
    framebuffer.width = create.width;
    framebuffer.height = create.height;
    framebuffer.pixel_format = DRM_FORMAT_XRGB8888;
    framebuffer.handles[0] = create.handle;
    framebuffer.pitches[0] = create.pitch;
    if (drm_ioctl(fd, DRM_IOCTL_MODE_ADDFB2, &framebuffer) < 0) {
        fprintf(stderr, "drm-splash: ADDFB2 failed: %s\n", strerror(errno));
        return 7;
    }

    struct drm_mode_map_dumb map_request = {0};
    map_request.handle = create.handle;
    if (drm_ioctl(fd, DRM_IOCTL_MODE_MAP_DUMB, &map_request) < 0) {
        fprintf(stderr, "drm-splash: MAP_DUMB failed: %s\n", strerror(errno));
        return 8;
    }
    uint32_t *pixels = mmap(NULL, create.size, PROT_READ | PROT_WRITE,
                            MAP_SHARED, fd, map_request.offset);
    if (pixels == MAP_FAILED) {
        fprintf(stderr, "drm-splash: mmap failed: %s\n", strerror(errno));
        return 9;
    }
    int selection = 0;
    bool active = false;
    bool display_on = true;
    uint64_t last_activity_ms = monotonic_milliseconds();
    int page = 0;
    render_launcher(pixels, create.pitch / sizeof(uint32_t),
                    create.width, create.height, selection, active);
    msync(pixels, create.size, MS_SYNC);

    struct drm_mode_crtc set_crtc = {0};
    active_connector_id = selected_connector;
    active_crtc_id = crtc_id;
    active_mode = mode;
    set_crtc.set_connectors_ptr = (uintptr_t)&selected_connector;
    set_crtc.count_connectors = 1;
    set_crtc.crtc_id = crtc_id;
    set_crtc.fb_id = framebuffer.fb_id;
    set_crtc.mode_valid = 1;
    set_crtc.mode = mode;
    if (drm_ioctl(fd, DRM_IOCTL_MODE_SETCRTC, &set_crtc) < 0) {
        fprintf(stderr, "drm-splash: SETCRTC failed: %s\n", strerror(errno));
        return 10;
    }

    fprintf(stderr,
            "drm-splash: SaaiOS splash active on connector %u, CRTC %u, FB %u\n",
            selected_connector, crtc_id, framebuffer.fb_id);

    static const char *const input_paths[] = {
        "/dev/input/touchscreen",
        "/dev/input/volume-buttons",
        "/dev/input/power-button",
    };
    struct pollfd inputs[3];
    int input_kind[3];
    int input_count = 0;
    for (int kind = 0; kind < 3; ++kind) {
        int input = open(input_paths[kind], O_RDONLY | O_CLOEXEC | O_NONBLOCK);
        if (input < 0) {
            fprintf(stderr, "drm-splash: input %s unavailable: %s\n",
                    input_paths[kind], strerror(errno));
            continue;
        }
        inputs[input_count].fd = input;
        inputs[input_count].events = POLLIN;
        input_kind[input_count] = kind;
        ++input_count;
    }
    fprintf(stderr, "drm-splash: launcher ready with %d input devices\n",
            input_count);
    setup_haptic();

    int tracking_id = -1;
    int touch_x = 0;
    int touch_y = 0;
    int bluetooth_touch_item = -1;
    int console_touch_item = -1;
    bool position_changed = false;
    bool touch_released = false;
    for (;;) {
        if (input_count == 0) {
            pause();
            continue;
        }
        int ready = poll(inputs, (nfds_t)input_count, 1000);
        if (ready < 0) {
            if (errno == EINTR) {
                continue;
            }
            fprintf(stderr, "drm-splash: input poll failed: %s\n",
                    strerror(errno));
            return 11;
        }
        if (ready == 0) {
            uint64_t now_ms = monotonic_milliseconds();
            if (display_on && now_ms != 0 && last_activity_ms != 0 &&
                now_ms - last_activity_ms >= 60000U) {
                fprintf(stderr, "drm-splash: idle timeout\n");
                disable_display(fd);
                display_on = false;
                continue;
            }
            if (display_on && page != 0) {
                render_page(pixels, create.pitch / sizeof(uint32_t),
                            create.width, create.height,
                            page, selection, active);
                publish_frame(fd, framebuffer.fb_id, pixels, create.size);
            }
            continue;
        }
        for (int index = 0; index < input_count; ++index) {
            if (!(inputs[index].revents & POLLIN)) {
                continue;
            }
            struct input_event event;
            ssize_t count = read(inputs[index].fd, &event, sizeof(event));
            if (count != (ssize_t)sizeof(event)) {
                continue;
            }
            if (input_kind[index] == 2 && event.type == EV_KEY &&
                event.code == KEY_POWER && event.value == 1) {
                last_activity_ms = monotonic_milliseconds();
                play_haptic();
                if (display_on) {
                    disable_display(fd);
                    display_on = false;
                } else {
                    publish_frame(fd, framebuffer.fb_id,
                                  pixels, create.size);
                    display_on = true;
                    fprintf(stderr, "drm-splash: display on\n");
                }
                continue;
            }
            if (!display_on) {
                continue;
            }
            if ((input_kind[index] == 0 &&
                 (event.type == EV_ABS || event.type == EV_SYN)) ||
                (event.type == EV_KEY && event.value == 1)) {
                last_activity_ms = monotonic_milliseconds();
            }
            if (input_kind[index] == 0) {
                if (event.type == EV_ABS) {
                    if (event.code == ABS_MT_TRACKING_ID) {
                        tracking_id = event.value;
                        touch_released = event.value < 0;
                    } else if (event.code == ABS_MT_POSITION_X) {
                        touch_x = event.value;
                        position_changed = true;
                    } else if (event.code == ABS_MT_POSITION_Y) {
                        touch_y = event.value;
                        position_changed = true;
                    }
                } else if (event.type == EV_SYN && event.code == SYN_REPORT) {
                    if (tracking_id >= 0 && position_changed) {
                        int touched_item = -1;
                        if (page == 0) {
                            touched_item = menu_item_at(touch_y);
                            if (touched_item >= 0 && touched_item != selection) {
                                selection = touched_item;
                                active = false;
                                render_launcher(pixels,
                                    create.pitch / sizeof(uint32_t),
                                    create.width, create.height, selection, active);
                            }
                        } else if (page == 4) {
                            bluetooth_touch_item = bluetooth_item_at(touch_y);
                            touched_item = bluetooth_touch_item;
                        } else if (page == 5) {
                            console_touch_item = console_item_at(touch_y);
                            touched_item = console_touch_item;
                        }
                        draw_touch_marker(pixels,
                            create.pitch / sizeof(uint32_t),
                            create.width, create.height, touch_x, touch_y);
                        publish_frame(fd, framebuffer.fb_id, pixels, create.size);
                        fprintf(stderr, "drm-splash: touch x=%d y=%d item=%d\n",
                                touch_x, touch_y, touched_item);
                        position_changed = false;
                    } else if (touch_released) {
                        play_haptic();
                        if (page == 4 && bluetooth_touch_item >= 0) {
                            if (bluetooth_saved_view) {
                                if (bluetooth_forget_candidate ==
                                    bluetooth_touch_item) {
                                    start_bluetooth_forget(
                                        bluetooth_touch_item);
                                    bluetooth_forget_candidate = -1;
                                } else {
                                    bluetooth_forget_candidate =
                                        bluetooth_touch_item;
                                }
                            } else {
                                start_bluetooth_pair(bluetooth_touch_item);
                            }
                            active = true;
                        } else if (page == 5 && console_touch_item >= 0) {
                            start_ai_query(console_touch_item);
                            active = true;
                        } else if (page != 0) {
                            page = 0;
                            active = false;
                            bluetooth_forget_candidate = -1;
                        } else {
                            active = true;
                            page = selection + 1;
                            if (page == 3) {
                                start_audio_test();
                            } else if (page == 4) {
                                bluetooth_saved_view = false;
                                bluetooth_forget_candidate = -1;
                                start_bluetooth_scan();
                            }
                        }
                        render_page(pixels, create.pitch / sizeof(uint32_t),
                            create.width, create.height, page, selection, active);
                        publish_frame(fd, framebuffer.fb_id, pixels, create.size);
                        fprintf(stderr, "drm-splash: activated item=%d\n",
                                selection);
                        bluetooth_touch_item = -1;
                        console_touch_item = -1;
                        touch_released = false;
                    }
                }
            } else if (event.type == EV_KEY && event.value == 1) {
                if (page != 0) {
                    if (page == 1 && event.code == KEY_VOLUMEUP) {
                        play_haptic();
                        adjust_display_brightness(true);
                        continue;
                    }
                    if (page == 1 && event.code == KEY_VOLUMEDOWN) {
                        play_haptic();
                        adjust_display_brightness(false);
                        continue;
                    }
                    if (page == 3 && event.code == KEY_VOLUMEUP) {
                        play_haptic();
                        adjust_audio_volume(true);
                        continue;
                    }
                    if (page == 3 && event.code == KEY_VOLUMEDOWN) {
                        play_haptic();
                        adjust_audio_volume(false);
                        continue;
                    }
                    if (page == 4 &&
                        (event.code == KEY_VOLUMEUP ||
                         event.code == KEY_VOLUMEDOWN)) {
                        play_haptic();
                        bluetooth_saved_view = !bluetooth_saved_view;
                        bluetooth_forget_candidate = -1;
                        render_bluetooth(pixels,
                            create.pitch / sizeof(uint32_t),
                            create.width, create.height);
                        publish_frame(fd, framebuffer.fb_id,
                                      pixels, create.size);
                        continue;
                    }
                    continue;
                } else if (event.code == KEY_VOLUMEUP) {
                    play_haptic();
                    selection = (selection + 4) % 5;
                    active = false;
                } else if (event.code == KEY_VOLUMEDOWN) {
                    play_haptic();
                    selection = (selection + 1) % 5;
                    active = false;
                } else {
                    continue;
                }
                render_page(pixels, create.pitch / sizeof(uint32_t),
                            create.width, create.height, page, selection, active);
                publish_frame(fd, framebuffer.fb_id, pixels, create.size);
                fprintf(stderr, "drm-splash: key=%u item=%d active=%d\n",
                        event.code, selection, active ? 1 : 0);
            }
        }
    }
}
