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
static bool ai_keyboard_open = false;
#define AI_PROMPT_MAX 72
static char ai_prompt[AI_PROMPT_MAX + 1] = {0};
static size_t ai_prompt_length = 0;
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
        case '<': return "00001" "00010" "00100" "01000" "00100" "00010" "00001";
        default:  return "00000" "00000" "00000" "00000" "00000" "00000" "00000";
    }
}

static int ui_x(uint32_t width, int value) {
    return (int)((int64_t)value * width / 1080);
}

static int ui_y(uint32_t height, int value) {
    return (int)((int64_t)value * height / 2400);
}

static int ui_scale(uint32_t width, int value) {
    int scaled = ui_x(width, value);
    return scaled > 0 ? scaled : 1;
}

static int text_width(const char *word, int scale) {
    size_t length = strlen(word);
    return length == 0 ? 0 : ((int)length * 6 - 1) * scale;
}

static void draw_text(uint32_t *pixels, uint32_t stride_pixels,
                      uint32_t width, uint32_t height,
                      const char *word, int scale, int origin_x,
                      int center_y, uint32_t color) {
    size_t length = strlen(word);
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

static void draw_word(uint32_t *pixels, uint32_t stride_pixels,
                      uint32_t width, uint32_t height,
                      const char *word, int scale, int center_y,
                      uint32_t color) {
    draw_text(pixels, stride_pixels, width, height, word, scale,
              ((int)width - text_width(word, scale)) / 2,
              center_y, color);
}

static void fill_soft_rect(uint32_t *pixels, uint32_t stride_pixels,
                           uint32_t width, uint32_t height,
                           int x, int y, int w, int h, int radius,
                           uint32_t color) {
    if (radius < 1) {
        fill_rect(pixels, stride_pixels, width, height, x, y, w, h, color);
        return;
    }
    fill_rect(pixels, stride_pixels, width, height,
              x + radius, y, w - radius * 2, h, color);
    fill_rect(pixels, stride_pixels, width, height,
              x, y + radius, w, h - radius * 2, color);
    fill_rect(pixels, stride_pixels, width, height,
              x + radius / 2, y + radius / 3,
              w - radius, h - radius * 2 / 3, color);
}

static bool read_first_line(const char *path, char *value, size_t value_size);
static bool read_wifi_address(char *address, size_t address_size);

static void render_status_bar(uint32_t *pixels, uint32_t stride_pixels,
                              uint32_t width, uint32_t height) {
    char clock_text[8] = "--:--";
    char capacity[8] = {0};
    char battery_text[12] = "--%";
    char address[INET_ADDRSTRLEN] = {0};
    time_t now = time(NULL);
    if (now > 1700000000) {
        struct tm utc = {0};
        if (gmtime_r(&now, &utc)) {
            (void)strftime(clock_text, sizeof(clock_text), "%H:%M", &utc);
        }
    }
    if (read_first_line("/sys/class/power_supply/maxfg/capacity",
                        capacity, sizeof(capacity))) {
        snprintf(battery_text, sizeof(battery_text), "%.3s%%", capacity);
    }
    int scale = ui_scale(width, 6);
    draw_text(pixels, stride_pixels, width, height,
              clock_text, scale, ui_x(width, 54), ui_y(height, 82),
              0x00E8F0FA);
    if (read_wifi_address(address, sizeof(address))) {
        fill_soft_rect(pixels, stride_pixels, width, height,
                       ui_x(width, 748), ui_y(height, 59),
                       ui_x(width, 54), ui_y(height, 38),
                       ui_x(width, 12), 0x0000CFA0);
    }
    int battery_scale = ui_scale(width, 6);
    draw_text(pixels, stride_pixels, width, height,
              battery_text, battery_scale,
              (int)width - ui_x(width, 54) -
                  text_width(battery_text, battery_scale),
              ui_y(height, 82), 0x00E8F0FA);
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

    render_status_bar(pixels, stride_pixels, width, height);
}

static void render_page_chrome(uint32_t *pixels, uint32_t stride_pixels,
                               uint32_t width, uint32_t height,
                               const char *title) {
    render_splash(pixels, stride_pixels, width, height);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 48), ui_y(height, 170),
                   ui_x(width, 170), ui_y(height, 96),
                   ui_x(width, 28), 0x001A2B46);
    draw_text(pixels, stride_pixels, width, height,
              "< BACK", ui_scale(width, 6), ui_x(width, 72),
              ui_y(height, 218), 0x00AFC3DA);
    draw_text(pixels, stride_pixels, width, height,
              title, ui_scale(width, 12), ui_x(width, 54),
              ui_y(height, 355), 0x00F5F8FC);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 390), ui_y(height, 2320),
                   ui_x(width, 300), ui_y(height, 18),
                   ui_x(width, 9), 0x007B91AA);
}

static void render_launcher(uint32_t *pixels, uint32_t stride_pixels,
                            uint32_t width, uint32_t height,
                            int selection, bool active) {
    static const char *const labels[] = {
        "STATUS", "NETWORK", "SOUND", "BLUETOOTH", "CONSOLE"
    };
    static const uint32_t accents[] = {
        0x004F8CFF, 0x0000CFA0, 0x00D786FF, 0x008779FF, 0x006C63FF
    };
    render_splash(pixels, stride_pixels, width, height);
    draw_text(pixels, stride_pixels, width, height,
              "SAAIOS", ui_scale(width, 14), ui_x(width, 54),
              ui_y(height, 280), 0x00F5F8FC);
    draw_text(pixels, stride_pixels, width, height,
              "YOUR PRIVATE PHONE", ui_scale(width, 6), ui_x(width, 58),
              ui_y(height, 405), 0x008EA8C6);
    for (int index = 0; index < 4; ++index) {
        int column = index % 2;
        int row = index / 2;
        int left = 54 + column * 501;
        int top = 520 + row * 445;
        uint32_t color = index == selection
            ? (active ? 0x002D5E85 : 0x00213E60)
            : 0x00152238;
        fill_soft_rect(pixels, stride_pixels, width, height,
                       ui_x(width, left), ui_y(height, top),
                       ui_x(width, 471), ui_y(height, 405),
                       ui_x(width, 42), color);
        fill_soft_rect(pixels, stride_pixels, width, height,
                       ui_x(width, left + 34), ui_y(height, top + 34),
                       ui_x(width, 104), ui_y(height, 104),
                       ui_x(width, 28), accents[index]);
        draw_text(pixels, stride_pixels, width, height,
                  labels[index], ui_scale(width, index == 3 ? 7 : 9),
                  ui_x(width, left + 34), ui_y(height, top + 300),
                  0x00F4F7FB);
    }
    uint32_t ai_color = selection == 4
        ? (active ? 0x004B4AC4 : 0x003A397F) : 0x00232158;
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 54), ui_y(height, 1450),
                   ui_x(width, 972), ui_y(height, 360),
                   ui_x(width, 48), ai_color);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 88), ui_y(height, 1490),
                   ui_x(width, 112), ui_y(height, 112),
                   ui_x(width, 32), accents[4]);
    draw_text(pixels, stride_pixels, width, height,
              "AI ASSISTANT", ui_scale(width, 11), ui_x(width, 88),
              ui_y(height, 1680), 0x00FFFFFF);
    draw_text(pixels, stride_pixels, width, height,
              "LOCAL AND PRIVATE", ui_scale(width, 6), ui_x(width, 91),
              ui_y(height, 1755), 0x00BDBEFF);
    draw_word(pixels, stride_pixels, width, height,
              "TAP AN APP", ui_scale(width, 7), ui_y(height, 2110),
              0x006E87A5);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 390), ui_y(height, 2320),
                   ui_x(width, 300), ui_y(height, 18),
                   ui_x(width, 9), 0x007B91AA);
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

static void start_ai_request(const char *prompt, int action) {
    if (!prompt || !prompt[0] || ai_query_running()) {
        return;
    }
    (void)unlink("/run/saaios-ai-ui.log");
    ai_last_action = action;
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
              "--ask", prompt, NULL);
        dprintf(STDOUT_FILENO, "MODEL OFFLINE\n");
        _exit(127);
    }
    if (child > 0) {
        ai_query_pid = child;
    }
}

static void start_ai_query(int selected) {
    static const char *const prompts[] = {
        "Use system.metrics exactly once. Reply with at most three short lines in uppercase ASCII English. Summarize CPU, free memory, and load. Do not call any other tool.",
        "Use network.status exactly once. Reply with at most three short lines in uppercase ASCII English. Summarize interfaces and connectivity. Do not call any other tool.",
        "Use system.disk exactly once. Reply with at most three short lines in uppercase ASCII English. Summarize total and free storage. Do not call any other tool."
    };
    if (selected < 0 || selected >= (int)(sizeof(prompts) / sizeof(prompts[0]))) {
        return;
    }
    start_ai_request(prompts[selected], selected);
}

static void start_ai_user_query(void) {
    if (ai_prompt_length == 0 || ai_query_running()) {
        return;
    }
    char request[320];
    snprintf(request, sizeof(request),
             "Reply in uppercase ASCII English using at most four short lines. "
             "Answer this user question directly: %s", ai_prompt);
    start_ai_request(request, 3);
}

static void render_networks(uint32_t *pixels, uint32_t stride_pixels,
                            uint32_t width, uint32_t height) {
    char names[3][32] = {{0}};
    char address[INET_ADDRSTRLEN] = {0};
    int count = read_network_names(names, 3);
    bool online = read_wifi_address(address, sizeof(address));
    render_page_chrome(pixels, stride_pixels, width, height, "NETWORK");
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 54), ui_y(height, 470),
                   ui_x(width, 972), ui_y(height, 290),
                   ui_x(width, 44), 0x0015253B);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 92), ui_y(height, 520),
                   ui_x(width, 72), ui_y(height, 72),
                   ui_x(width, 24),
                   online ? 0x0000CFA0 : 0x005F7691);
    draw_text(pixels, stride_pixels, width, height,
              online ? "WIFI CONNECTED" : "WIFI READY",
              ui_scale(width, 9), ui_x(width, 200),
              ui_y(height, 565), 0x00F5F8FC);
    if (online) {
        draw_text(pixels, stride_pixels, width, height,
                  address, ui_scale(width, 8), ui_x(width, 92),
                  ui_y(height, 685), 0x009CB1C9);
    }
    draw_text(pixels, stride_pixels, width, height,
              "NEARBY NETWORKS", ui_scale(width, 7), ui_x(width, 58),
              ui_y(height, 865), 0x008EA8C6);
    if (count == 0) {
        fill_soft_rect(pixels, stride_pixels, width, height,
                       ui_x(width, 54), ui_y(height, 930),
                       ui_x(width, 972), ui_y(height, 230),
                       ui_x(width, 36), 0x00152238);
        draw_word(pixels, stride_pixels, width, height,
                  "SCANNING", ui_scale(width, 9), ui_y(height, 1045),
                  0x008EA8C6);
    } else {
        for (int index = 0; index < count; ++index) {
            int top = 930 + index * 260;
            fill_soft_rect(pixels, stride_pixels, width, height,
                           ui_x(width, 54), ui_y(height, top),
                           ui_x(width, 972), ui_y(height, 220),
                           ui_x(width, 36), 0x00152238);
            fill_soft_rect(pixels, stride_pixels, width, height,
                           ui_x(width, 90), ui_y(height, top + 72),
                           ui_x(width, 54), ui_y(height, 54),
                           ui_x(width, 18), 0x004F8CFF);
            int length = (int)strlen(names[index]);
            int scale = length > 15 ? 7 : (length > 11 ? 8 : 9);
            draw_text(pixels, stride_pixels, width, height,
                      names[index], ui_scale(width, scale), ui_x(width, 185),
                      ui_y(height, top + 110), 0x00F5F8FC);
        }
    }
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
    render_page_chrome(pixels, stride_pixels, width, height, "BLUETOOTH");
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 54), ui_y(height, 455),
                   ui_x(width, 972), ui_y(height, 126),
                   ui_x(width, 34), 0x00152238);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, bluetooth_saved_view ? 540 : 54),
                   ui_y(height, 455), ui_x(width, 486), ui_y(height, 126),
                   ui_x(width, 34), 0x0037567D);
    draw_text(pixels, stride_pixels, width, height,
              "NEARBY", ui_scale(width, 7), ui_x(width, 190),
              ui_y(height, 518), bluetooth_saved_view ? 0x007E96B2 : 0x00FFFFFF);
    draw_text(pixels, stride_pixels, width, height,
              "SAVED", ui_scale(width, 7), ui_x(width, 710),
              ui_y(height, 518), bluetooth_saved_view ? 0x00FFFFFF : 0x007E96B2);
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
              status, ui_scale(width, 7), ui_y(height, 665), status_color);
    if (count == 0) {
        draw_word(pixels, stride_pixels, width, height,
                  bluetooth_saved_view ? "NO SAVED DEVICES" :
                  (scanning ? "LOOKING FOR DEVICES" :
                  (finished ? "NO NAMED DEVICES" : "BLUETOOTH READY")),
                  ui_scale(width, 8), ui_y(height, 1230), 0x008CA9C8);
    } else {
        for (int index = 0; index < count; ++index) {
            int top = 760 + index * 245;
            uint32_t card_color = 0x00152238;
            if (bluetooth_saved_view &&
                bluetooth_forget_candidate == index) {
                card_color = 0x00804343;
            } else if (!bluetooth_saved_view && pair_state == 2 &&
                       !strcmp(pair_name, names[index])) {
                card_color = 0x00006F5A;
            }
            fill_soft_rect(pixels, stride_pixels, width, height,
                           ui_x(width, 54), ui_y(height, top),
                           ui_x(width, 972), ui_y(height, 195),
                           ui_x(width, 36), card_color);
            fill_soft_rect(pixels, stride_pixels, width, height,
                           ui_x(width, 88), ui_y(height, top + 54),
                           ui_x(width, 72), ui_y(height, 72),
                           ui_x(width, 22), 0x008779FF);
            int length = (int)strlen(names[index]);
            int scale = length > 15 ? 7 : (length > 11 ? 8 : 9);
            draw_text(pixels, stride_pixels, width, height,
                      names[index], ui_scale(width, scale), ui_x(width, 205),
                      ui_y(height, top + 98), 0x00FFFFFF);
        }
    }
}

static int bluetooth_item_at(int y, uint32_t height) {
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
    int design_y = (int)((int64_t)y * 2400 / height);
    for (int index = 0; index < count; ++index) {
        int top = 760 + index * 245;
        if (design_y >= top && design_y < top + 195) {
            return index;
        }
    }
    return -1;
}

static void render_dashboard_card(uint32_t *pixels, uint32_t stride_pixels,
                                  uint32_t width, uint32_t height,
                                  int left, int top, const char *label,
                                  const char *value, uint32_t accent) {
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, left), ui_y(height, top),
                   ui_x(width, 471), ui_y(height, 350),
                   ui_x(width, 42), 0x00152238);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, left + 32), ui_y(height, top + 32),
                   ui_x(width, 68), ui_y(height, 68),
                   ui_x(width, 22), accent);
    draw_text(pixels, stride_pixels, width, height,
              label, ui_scale(width, 6), ui_x(width, left + 32),
              ui_y(height, top + 150), 0x008EA8C6);
    int value_scale = strlen(value) > 10 ? 6 : 9;
    draw_text(pixels, stride_pixels, width, height,
              value, ui_scale(width, value_scale), ui_x(width, left + 32),
              ui_y(height, top + 255), 0x00F5F8FC);
}

static void render_status(uint32_t *pixels, uint32_t stride_pixels,
                          uint32_t width, uint32_t height) {
    char address[INET_ADDRSTRLEN] = {0};
    char capacity[8] = {0};
    char battery[12] = "--%";
    char raw_brightness[16] = {0};
    char brightness[12] = "25%";
    bool online = read_wifi_address(address, sizeof(address));
    if (read_first_line("/sys/class/power_supply/maxfg/capacity",
                        capacity, sizeof(capacity))) {
        snprintf(battery, sizeof(battery), "%.3s%%", capacity);
    }
    if (read_first_line(
            "/sys/devices/platform/1c2c0000.drmdsim/"
            "1c2c0000.drmdsim.0/backlight/panel0-backlight/brightness",
            raw_brightness, sizeof(raw_brightness))) {
        int percent = atoi(raw_brightness) * 100 / 4095;
        if (percent < 0) { percent = 0; }
        if (percent > 100) { percent = 100; }
        snprintf(brightness, sizeof(brightness), "%d%%", percent);
    }
    bool data_ready = access("/data/saaios/.layout", R_OK) == 0;
    bool audio_ready = access("/run/audio-ready", R_OK) == 0;
    bool bluetooth_ready =
        access("/sys/class/bluetooth/hci0", R_OK) == 0;
    render_page_chrome(pixels, stride_pixels, width, height, "OVERVIEW");
    draw_text(pixels, stride_pixels, width, height,
              "SYSTEM READY", ui_scale(width, 7), ui_x(width, 58),
              ui_y(height, 460), 0x0000D6A3);
    render_dashboard_card(pixels, stride_pixels, width, height,
                          54, 540, "BATTERY", battery, 0x0000CFA0);
    render_dashboard_card(pixels, stride_pixels, width, height,
                          555, 540, "DISPLAY", brightness, 0x00D786FF);
    render_dashboard_card(pixels, stride_pixels, width, height,
                          54, 925, "NETWORK",
                          online ? "ONLINE" : "OFFLINE", 0x004F8CFF);
    render_dashboard_card(pixels, stride_pixels, width, height,
                          555, 925, "STORAGE",
                          data_ready ? "READY" : "OFFLINE", 0x006C63FF);
    render_dashboard_card(pixels, stride_pixels, width, height,
                          54, 1310, "AUDIO",
                          audio_ready ? "READY" : "STARTING", 0x00D786FF);
    render_dashboard_card(pixels, stride_pixels, width, height,
                          555, 1310, "BLUETOOTH",
                          bluetooth_ready ? "READY" : "STARTING", 0x008779FF);
    draw_text(pixels, stride_pixels, width, height,
              "VOLUME KEYS CONTROL BRIGHTNESS", ui_scale(width, 5),
              ui_x(width, 58), ui_y(height, 1775), 0x006E87A5);
    if (online) {
        draw_text(pixels, stride_pixels, width, height,
                  address, ui_scale(width, 7), ui_x(width, 58),
                  ui_y(height, 1880), 0x009CB1C9);
    }
}

static void draw_keyboard_key(uint32_t *pixels, uint32_t stride_pixels,
                              uint32_t width, uint32_t height,
                              int left, int top, int key_width, int key_height,
                              const char *label, int label_scale,
                              uint32_t color) {
    int x = ui_x(width, left);
    int w = ui_x(width, key_width);
    int scale = ui_scale(width, label_scale);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   x, ui_y(height, top), w, ui_y(height, key_height),
                   ui_x(width, 22), color);
    draw_text(pixels, stride_pixels, width, height,
              label, scale, x + (w - text_width(label, scale)) / 2,
              ui_y(height, top + key_height / 2), 0x00F5F8FC);
}

static void render_keyboard(uint32_t *pixels, uint32_t stride_pixels,
                            uint32_t width, uint32_t height) {
    static const char *const rows[] = {
        "QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"
    };
    static const int row_left[] = {30, 79, 174};
    static const int row_top[] = {950, 1130, 1310};
    static const int row_gap[] = {8, 10, 12};
    render_page_chrome(pixels, stride_pixels, width, height, "NEW QUESTION");
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 54), ui_y(height, 455),
                   ui_x(width, 972), ui_y(height, 340),
                   ui_x(width, 42), 0x00152238);
    draw_text(pixels, stride_pixels, width, height,
              "YOUR QUESTION", ui_scale(width, 6), ui_x(width, 88),
              ui_y(height, 520), 0x008EA8C6);
    if (ai_prompt_length == 0) {
        draw_text(pixels, stride_pixels, width, height,
                  "TYPE WITH THE KEYS BELOW", ui_scale(width, 6),
                  ui_x(width, 88), ui_y(height, 650), 0x006E87A5);
    } else {
        for (int line = 0; line < 3; ++line) {
            size_t offset = (size_t)line * 24;
            if (offset >= ai_prompt_length) break;
            char text[25] = {0};
            size_t remaining = ai_prompt_length - offset;
            size_t count = remaining > 24 ? 24 : remaining;
            memcpy(text, ai_prompt + offset, count);
            draw_text(pixels, stride_pixels, width, height,
                      text, ui_scale(width, 6), ui_x(width, 88),
                      ui_y(height, 615 + line * 88), 0x00F5F8FC);
        }
    }
    static const char digits[] = "1234567890";
    for (int column = 0; column < 10; ++column) {
        char label[2] = {digits[column], '\0'};
        draw_keyboard_key(pixels, stride_pixels, width, height,
                          30 + column * 102, 820, 94, 100,
                          label, 5, 0x001A2B46);
    }
    for (int row = 0; row < 3; ++row) {
        size_t count = strlen(rows[row]);
        for (size_t column = 0; column < count; ++column) {
            char label[2] = {rows[row][column], '\0'};
            draw_keyboard_key(pixels, stride_pixels, width, height,
                              row_left[row] + (int)column * (94 + row_gap[row]),
                              row_top[row], 94, 150, label, 7, 0x00213655);
        }
    }
    draw_keyboard_key(pixels, stride_pixels, width, height,
                      54, 1490, 650, 160, "SPACE", 7, 0x00213655);
    draw_keyboard_key(pixels, stride_pixels, width, height,
                      724, 1490, 302, 160, "DELETE", 6, 0x00334A67);
    draw_keyboard_key(pixels, stride_pixels, width, height,
                      54, 1710, 470, 170, "CANCEL", 7, 0x00213655);
    draw_keyboard_key(pixels, stride_pixels, width, height,
                      546, 1710, 480, 170, "SEND", 8,
                      ai_prompt_length > 0 ? 0x006C63FF : 0x002B3154);
    draw_word(pixels, stride_pixels, width, height,
              "LATIN KEYBOARD", ui_scale(width, 6), ui_y(height, 1990),
              0x006E87A5);
}

static void render_console(uint32_t *pixels, uint32_t stride_pixels,
                           uint32_t width, uint32_t height) {
    static const char *const labels[] = {
        "SYSTEM HEALTH", "NETWORK CHECK", "STORAGE CHECK"
    };
    bool running = ai_query_running();
    char lines[4][AI_LINE_CHARS + 1] = {{0}};
    int line_count = running ? 0 : read_ai_lines(lines, 4);
    render_page_chrome(pixels, stride_pixels, width, height, "ASSISTANT");
    bool runtime_ready = access("/tmp/saaios.sock", F_OK) == 0;
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 54), ui_y(height, 455),
                   ui_x(width, 972), ui_y(height, 110),
                   ui_x(width, 32), 0x00152238);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 88), ui_y(height, 488),
                   ui_x(width, 44), ui_y(height, 44),
                   ui_x(width, 14), runtime_ready ? 0x0000CFA0 : 0x00D56D6D);
    draw_text(pixels, stride_pixels, width, height,
              runtime_ready ? "LOCAL AI READY" : "RUNTIME OFFLINE",
              ui_scale(width, 7), ui_x(width, 170), ui_y(height, 510),
              runtime_ready ? 0x00C8F7EA : 0x00FFD0D0);

    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 54), ui_y(height, 620),
                   ui_x(width, 972), ui_y(height, 220),
                   ui_x(width, 40), ai_last_action == 3 ? 0x003A397F : 0x00232158);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 88), ui_y(height, 674),
                   ui_x(width, 86), ui_y(height, 86),
                   ui_x(width, 26), 0x006C63FF);
    draw_text(pixels, stride_pixels, width, height,
              "ASK A QUESTION", ui_scale(width, 9), ui_x(width, 215),
              ui_y(height, 730), 0x00FFFFFF);

    draw_text(pixels, stride_pixels, width, height,
              "QUICK CHECKS", ui_scale(width, 6), ui_x(width, 58),
              ui_y(height, 905), 0x008EA8C6);
    for (int index = 0; index < 3; ++index) {
        int top = 950 + index * 190;
        uint32_t color = index == ai_last_action
            ? 0x003A397F : 0x00152238;
        fill_soft_rect(pixels, stride_pixels, width, height,
                       ui_x(width, 54), ui_y(height, top),
                       ui_x(width, 972), ui_y(height, 150),
                       ui_x(width, 32), color);
        fill_soft_rect(pixels, stride_pixels, width, height,
                       ui_x(width, 88), ui_y(height, top + 42),
                       ui_x(width, 66), ui_y(height, 66),
                       ui_x(width, 20), 0x006C63FF);
        draw_text(pixels, stride_pixels, width, height,
                  labels[index], ui_scale(width, 7), ui_x(width, 195),
                  ui_y(height, top + 75), 0x00FFFFFF);
    }
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 54), ui_y(height, 1540),
                   ui_x(width, 972), ui_y(height, 570),
                   ui_x(width, 44), 0x00101C2E);
    draw_text(pixels, stride_pixels, width, height,
              "AI RESPONSE", ui_scale(width, 6), ui_x(width, 92),
              ui_y(height, 1610), 0x007E96B2);
    if (running) {
        draw_word(pixels, stride_pixels, width, height,
                  "AI THINKING", ui_scale(width, 9), ui_y(height, 1810),
                  0x008C86FF);
    } else if (line_count > 0) {
        for (int index = 0; index < line_count; ++index) {
            int scale = strlen(lines[index]) > 22 ? 6 : 7;
            draw_text(pixels, stride_pixels, width, height,
                      lines[index], ui_scale(width, scale), ui_x(width, 92),
                      ui_y(height, 1725 + index * 105), 0x00B7CBE2);
        }
    } else {
        draw_word(pixels, stride_pixels, width, height,
                  "ASK ANYTHING", ui_scale(width, 7),
                  ui_y(height, 1810), 0x008CA9C8);
    }
}

static void render_sound(uint32_t *pixels, uint32_t stride_pixels,
                         uint32_t width, uint32_t height) {
    bool ready = access("/run/audio-ready", R_OK) == 0;
    bool playing = access("/run/audio-playing", R_OK) == 0;
    char raw_volume[16] = {0};
    char volume_label[24] = "VOLUME 50";
    int volume_percent = 50;
    if (read_first_line("/run/audio-volume",
                        raw_volume, sizeof(raw_volume))) {
        int value = atoi(raw_volume);
        volume_percent = (value - 400) * 100 / 417;
        if (volume_percent < 0) { volume_percent = 0; }
        if (volume_percent > 100) { volume_percent = 100; }
        snprintf(volume_label, sizeof(volume_label),
                 "VOLUME %d", volume_percent);
    }
    render_page_chrome(pixels, stride_pixels, width, height, "SOUND");
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 54), ui_y(height, 485),
                   ui_x(width, 972), ui_y(height, 980),
                   ui_x(width, 54), 0x00152238);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 390), ui_y(height, 620),
                   ui_x(width, 300), ui_y(height, 300),
                   ui_x(width, 74), playing ? 0x006C63FF : 0x002D5E85);
    draw_word(pixels, stride_pixels, width, height,
              playing ? "PLAYING" : "TEST", ui_scale(width, 10),
              ui_y(height, 770), 0x00FFFFFF);
    draw_word(pixels, stride_pixels, width, height,
              ready ? "AUDIO READY" : "AUDIO STARTING", ui_scale(width, 7),
              ui_y(height, 1040), ready ? 0x0000D6A3 : 0x008EA8C6);
    draw_word(pixels, stride_pixels, width, height,
              volume_label, ui_scale(width, 11), ui_y(height, 1190),
              0x00F5F8FC);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 180), ui_y(height, 1300),
                   ui_x(width, 720), ui_y(height, 28),
                   ui_x(width, 14), 0x00263A55);
    fill_soft_rect(pixels, stride_pixels, width, height,
                   ui_x(width, 180), ui_y(height, 1300),
                   ui_x(width, 720 * volume_percent / 100), ui_y(height, 28),
                   ui_x(width, 14), 0x00D786FF);
    draw_word(pixels, stride_pixels, width, height,
              "VOLUME KEYS", ui_scale(width, 7), ui_y(height, 1600),
              0x008EA8C6);
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
        if (ai_keyboard_open) {
            render_keyboard(pixels, stride_pixels, width, height);
        } else {
            render_console(pixels, stride_pixels, width, height);
        }
    } else {
        render_launcher(pixels, stride_pixels, width, height,
                        selection, active);
    }
}

static void draw_touch_marker(uint32_t *pixels, uint32_t stride_pixels,
                              uint32_t width, uint32_t height, int x, int y) {
    const int outer = ui_scale(width, 54);
    const int inner = ui_scale(width, 22);
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

static int menu_item_at(int x, int y, uint32_t width, uint32_t height) {
    int design_x = (int)((int64_t)x * 1080 / width);
    int design_y = (int)((int64_t)y * 2400 / height);
    for (int index = 0; index < 4; ++index) {
        int left = 54 + (index % 2) * 501;
        int top = 520 + (index / 2) * 445;
        if (design_x >= left && design_x < left + 471 &&
            design_y >= top && design_y < top + 405) {
            return index;
        }
    }
    if (design_x >= 54 && design_x < 1026 &&
        design_y >= 1450 && design_y < 1810) {
        return 4;
    }
    return -1;
}

static int console_item_at(int y, uint32_t height) {
    int design_y = (int)((int64_t)y * 2400 / height);
    if (design_y >= 620 && design_y < 840) {
        return 3;
    }
    for (int index = 0; index < 3; ++index) {
        int top = 950 + index * 190;
        if (design_y >= top && design_y < top + 150) {
            return index;
        }
    }
    return -1;
}

static int bluetooth_tab_at(int x, int y,
                            uint32_t width, uint32_t height) {
    int design_x = (int)((int64_t)x * 1080 / width);
    int design_y = (int)((int64_t)y * 2400 / height);
    if (design_y < 455 || design_y >= 581 ||
        design_x < 54 || design_x >= 1026) {
        return -1;
    }
    return design_x < 540 ? 0 : 1;
}

static bool sound_action_at(int x, int y,
                            uint32_t width, uint32_t height) {
    int design_x = (int)((int64_t)x * 1080 / width);
    int design_y = (int)((int64_t)y * 2400 / height);
    return design_x >= 54 && design_x < 1026 &&
           design_y >= 485 && design_y < 1465;
}

static int keyboard_action_at(int x, int y,
                              uint32_t width, uint32_t height) {
    static const int row_left[] = {30, 79, 174};
    static const int row_top[] = {950, 1130, 1310};
    static const int row_gap[] = {8, 10, 12};
    static const int row_count[] = {10, 9, 7};
    static const int row_offset[] = {0, 10, 19};
    int design_x = (int)((int64_t)x * 1080 / width);
    int design_y = (int)((int64_t)y * 2400 / height);
    if (design_y >= 820 && design_y < 920) {
        for (int column = 0; column < 10; ++column) {
            int left = 30 + column * 102;
            if (design_x >= left && design_x < left + 94) {
                return 30 + column;
            }
        }
    }
    for (int row = 0; row < 3; ++row) {
        if (design_y < row_top[row] || design_y >= row_top[row] + 150) {
            continue;
        }
        for (int column = 0; column < row_count[row]; ++column) {
            int left = row_left[row] + column * (94 + row_gap[row]);
            if (design_x >= left && design_x < left + 94) {
                return row_offset[row] + column;
            }
        }
    }
    if (design_y >= 1490 && design_y < 1650) {
        if (design_x >= 54 && design_x < 704) return 26;
        if (design_x >= 724 && design_x < 1026) return 27;
    }
    if (design_y >= 1710 && design_y < 1880) {
        if (design_x >= 54 && design_x < 524) return 28;
        if (design_x >= 546 && design_x < 1026) return 29;
    }
    return -1;
}

static bool top_back_at(int x, int y,
                        uint32_t width, uint32_t height) {
    int design_x = (int)((int64_t)x * 1080 / width);
    int design_y = (int)((int64_t)y * 2400 / height);
    return design_x >= 38 && design_x < 240 &&
           design_y >= 150 && design_y < 310;
}

static bool home_at(int y, uint32_t height) {
    int design_y = (int)((int64_t)y * 2400 / height);
    return design_y >= 2260;
}

static bool page_back_at(int x, int y,
                         uint32_t width, uint32_t height) {
    return top_back_at(x, y, width, height) || home_at(y, height);
}

static void clear_ai_prompt(void) {
    ai_prompt_length = 0;
    ai_prompt[0] = '\0';
}

static void apply_keyboard_action(int action) {
    static const char letters[] = "QWERTYUIOPASDFGHJKLZXCVBNM";
    static const char digits[] = "1234567890";
    if (action >= 0 && action < 26 && ai_prompt_length < AI_PROMPT_MAX) {
        ai_prompt[ai_prompt_length++] = letters[action];
        ai_prompt[ai_prompt_length] = '\0';
    } else if (action >= 30 && action < 40 &&
               ai_prompt_length < AI_PROMPT_MAX) {
        ai_prompt[ai_prompt_length++] = digits[action - 30];
        ai_prompt[ai_prompt_length] = '\0';
    } else if (action == 26 && ai_prompt_length > 0 &&
               ai_prompt_length < AI_PROMPT_MAX &&
               ai_prompt[ai_prompt_length - 1] != ' ') {
        ai_prompt[ai_prompt_length++] = ' ';
        ai_prompt[ai_prompt_length] = '\0';
    } else if (action == 27 && ai_prompt_length > 0) {
        ai_prompt[--ai_prompt_length] = '\0';
    } else if (action == 28) {
        clear_ai_prompt();
        ai_keyboard_open = false;
    } else if (action == 29 && ai_prompt_length > 0 &&
               !ai_query_running()) {
        start_ai_user_query();
        clear_ai_prompt();
        ai_keyboard_open = false;
    }
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
    int launcher_touch_item = -1;
    int bluetooth_touch_item = -1;
    int bluetooth_touch_tab = -1;
    int console_touch_item = -1;
    int keyboard_touch_action = -1;
    bool sound_touch_action = false;
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
                        launcher_touch_item = -1;
                        bluetooth_touch_item = -1;
                        bluetooth_touch_tab = -1;
                        console_touch_item = -1;
                        keyboard_touch_action = -1;
                        sound_touch_action = false;
                        if (page == 0) {
                            launcher_touch_item = menu_item_at(
                                touch_x, touch_y, create.width, create.height);
                            touched_item = launcher_touch_item;
                            if (touched_item >= 0 && touched_item != selection) {
                                selection = touched_item;
                                active = false;
                                render_launcher(pixels,
                                    create.pitch / sizeof(uint32_t),
                                    create.width, create.height, selection, active);
                            }
                        } else if (page == 3) {
                            sound_touch_action = sound_action_at(
                                touch_x, touch_y, create.width, create.height);
                            touched_item = sound_touch_action ? 0 : -1;
                        } else if (page == 4) {
                            bluetooth_touch_tab = bluetooth_tab_at(
                                touch_x, touch_y, create.width, create.height);
                            if (bluetooth_touch_tab >= 0) {
                                touched_item = 100 + bluetooth_touch_tab;
                            } else {
                                bluetooth_touch_item = bluetooth_item_at(
                                    touch_y, create.height);
                                touched_item = bluetooth_touch_item;
                            }
                        } else if (page == 5) {
                            if (ai_keyboard_open) {
                                keyboard_touch_action = keyboard_action_at(
                                    touch_x, touch_y,
                                    create.width, create.height);
                                touched_item = keyboard_touch_action;
                            } else {
                                console_touch_item = console_item_at(
                                    touch_y, create.height);
                                touched_item = console_touch_item;
                            }
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
                        if (page == 3 && sound_touch_action) {
                            start_audio_test();
                            active = true;
                        } else if (page == 4 && bluetooth_touch_tab >= 0) {
                            bluetooth_saved_view = bluetooth_touch_tab == 1;
                            bluetooth_forget_candidate = -1;
                            if (!bluetooth_saved_view) {
                                start_bluetooth_scan();
                            }
                            active = true;
                        } else if (page == 4 && bluetooth_touch_item >= 0) {
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
                        } else if (page == 5 && ai_keyboard_open &&
                                   keyboard_touch_action >= 0) {
                            apply_keyboard_action(keyboard_touch_action);
                            active = true;
                        } else if (page == 5 && ai_keyboard_open &&
                                   top_back_at(touch_x, touch_y,
                                               create.width, create.height)) {
                            clear_ai_prompt();
                            ai_keyboard_open = false;
                            active = true;
                        } else if (page == 5 && !ai_keyboard_open &&
                                   console_touch_item >= 0) {
                            if (console_touch_item == 3) {
                                if (!ai_query_running()) {
                                    clear_ai_prompt();
                                    ai_keyboard_open = true;
                                }
                            } else {
                                start_ai_query(console_touch_item);
                            }
                            active = true;
                        } else if (page != 0 && page_back_at(
                                       touch_x, touch_y,
                                       create.width, create.height)) {
                            if (page == 5) {
                                clear_ai_prompt();
                                ai_keyboard_open = false;
                            }
                            page = 0;
                            active = false;
                            bluetooth_forget_candidate = -1;
                        } else if (page == 0 && launcher_touch_item >= 0) {
                            selection = launcher_touch_item;
                            active = true;
                            page = selection + 1;
                            if (page == 4) {
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
                        launcher_touch_item = -1;
                        bluetooth_touch_item = -1;
                        bluetooth_touch_tab = -1;
                        console_touch_item = -1;
                        keyboard_touch_action = -1;
                        sound_touch_action = false;
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
