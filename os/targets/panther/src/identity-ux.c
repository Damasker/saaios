#define _GNU_SOURCE

#include "identity-ux.h"

#include <ctype.h>
#include <stdio.h>
#include <string.h>

void identity_ux_reset(identity_ux *ux) {
    if (!ux) {
        return;
    }
    memset(ux, 0, sizeof(*ux));
    ux->state = IDENTITY_UX_IDLE;
    ux->error = IDENTITY_UX_ERR_NONE;
}

bool identity_ux_begin(identity_ux *ux, uint64_t now_ms) {
    if (!ux || ux->state == IDENTITY_UX_RUNNING) {
        return false;
    }
    ux->generation += 1U;
    ux->started_ms = now_ms;
    ux->state = IDENTITY_UX_RUNNING;
    ux->error = IDENTITY_UX_ERR_NONE;
    ux->product_line[0] = '\0';
    ux->device_line[0] = '\0';
    ux->slot_line[0] = '\0';
    return true;
}

bool identity_ux_matches(const identity_ux *ux, uint64_t generation) {
    return ux && ux->generation == generation;
}

void identity_ux_fail(identity_ux *ux, uint64_t generation,
                      identity_ux_error error) {
    if (!ux || !identity_ux_matches(ux, generation) ||
        ux->state != IDENTITY_UX_RUNNING) {
        return;
    }
    ux->state = IDENTITY_UX_ERROR;
    ux->error = error;
    ux->product_line[0] = '\0';
    ux->device_line[0] = '\0';
    ux->slot_line[0] = '\0';
}

bool identity_ux_target_token_ok(const char *token) {
    size_t length;
    size_t index;
    if (!token || !token[0]) {
        return false;
    }
    length = strlen(token);
    if (length == 0 || length > 12) {
        return false;
    }
    for (index = 0; index < length; ++index) {
        unsigned char character = (unsigned char)token[index];
        if (!(isalnum(character) || character == '_' || character == '-')) {
            return false;
        }
        if (islower(character)) {
            return false;
        }
    }
    return true;
}

static void copy_line(char *destination, const char *source) {
    size_t index = 0;
    if (!destination) {
        return;
    }
    if (!source) {
        destination[0] = '\0';
        return;
    }
    while (source[index] && index < IDENTITY_UX_LINE_CHARS) {
        destination[index] = source[index];
        ++index;
    }
    destination[index] = '\0';
}

void identity_ux_normalize_fields(const char *system_token,
                                  const char *class_token,
                                  const char *target_token,
                                  const char *slot_token,
                                  char *product_line,
                                  char *device_line,
                                  char *slot_line) {
    (void)class_token;
    /* Spec: product is fixed SAAIOS only if confirmed by the response. */
    if (system_token && strcmp(system_token, "SAAIOS") == 0) {
        copy_line(product_line, "SAAIOS");
    } else {
        product_line[0] = '\0';
    }

    if (target_token && strcmp(target_token, "PANTHER") == 0) {
        copy_line(device_line, "PIXEL 7 / PANTHER");
    } else if (identity_ux_target_token_ok(target_token)) {
        char combined[IDENTITY_UX_LINE_CHARS + 1];
        snprintf(combined, sizeof(combined), "DEVICE / %s", target_token);
        copy_line(device_line, combined);
    } else {
        copy_line(device_line, "DEVICE / TARGET UNKNOWN");
    }

    if (slot_token && (strcmp(slot_token, "A") == 0 ||
                       strcmp(slot_token, "B") == 0)) {
        char combined[IDENTITY_UX_LINE_CHARS + 1];
        snprintf(combined, sizeof(combined), "SLOT %s", slot_token);
        copy_line(slot_line, combined);
    } else {
        copy_line(slot_line, "SLOT UNKNOWN");
    }
}

static void uppercase_copy(char *destination, size_t capacity,
                           const char *source) {
    size_t index = 0;
    if (!destination || capacity == 0) {
        return;
    }
    if (!source) {
        destination[0] = '\0';
        return;
    }
    while (source[index] && index + 1 < capacity) {
        unsigned char character = (unsigned char)source[index];
        destination[index] = (char)toupper(character);
        ++index;
    }
    destination[index] = '\0';
}

static bool contains_words(const char *haystack, const char *needle) {
    return haystack && needle && strstr(haystack, needle) != NULL;
}

bool identity_ux_complete_from_output(identity_ux *ux, uint64_t generation,
                                      const char *raw_text) {
    char flat[768];
    size_t length = 0;
    bool pending_space = false;
    size_t source_index;
    char system_token[32] = {0};
    char class_token[32] = {0};
    char target_token[32] = {0};
    char slot_token[8] = {0};
    int assigned = 0;

    if (!ux || !identity_ux_matches(ux, generation) ||
        ux->state != IDENTITY_UX_RUNNING) {
        return false;
    }

    if (!raw_text) {
        identity_ux_fail(ux, generation, IDENTITY_UX_ERR_INVALID);
        return true;
    }

    for (source_index = 0;
         raw_text[source_index] && length + 1 < sizeof(flat);
         ++source_index) {
        unsigned char character = (unsigned char)raw_text[source_index];
        char mapped = '\0';
        if (character >= 'a' && character <= 'z') {
            mapped = (char)(character - 'a' + 'A');
        } else if ((character >= 'A' && character <= 'Z') ||
                   (character >= '0' && character <= '9') ||
                   character == '_' || character == '-') {
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
    flat[length] = '\0';

    if (contains_words(flat, "SYSTEM CORE OFFLINE") ||
        contains_words(flat, "RUNTIME UNAVAILABLE") ||
        contains_words(flat, "CONNECTION REFUSED")) {
        identity_ux_fail(ux, generation, IDENTITY_UX_ERR_OFFLINE);
        return true;
    }
    if (contains_words(flat, "IDENTITY TOOL MISSING") ||
        contains_words(flat, "TOOL MISSING") ||
        contains_words(flat, "UNKNOWN TOOL")) {
        identity_ux_fail(ux, generation, IDENTITY_UX_ERR_TOOL_MISSING);
        return true;
    }
    if (length == 0) {
        identity_ux_fail(ux, generation, IDENTITY_UX_ERR_INVALID);
        return true;
    }

    {
        char *save = NULL;
        for (char *word = strtok_r(flat, " ", &save); word;
             word = strtok_r(NULL, " ", &save)) {
            if (strcmp(word, "TARGET") == 0) {
                char *value = strtok_r(NULL, " ", &save);
                if (value) {
                    uppercase_copy(target_token, sizeof(target_token), value);
                }
                continue;
            }
            if (strcmp(word, "BOOT") == 0) {
                char *slot_word = strtok_r(NULL, " ", &save);
                char *value = strtok_r(NULL, " ", &save);
                if (slot_word && strcmp(slot_word, "SLOT") == 0 && value) {
                    uppercase_copy(slot_token, sizeof(slot_token), value);
                }
                continue;
            }
            if (strcmp(word, "SLOT") == 0) {
                char *value = strtok_r(NULL, " ", &save);
                if (value) {
                    uppercase_copy(slot_token, sizeof(slot_token), value);
                }
                continue;
            }
            if (assigned == 0) {
                uppercase_copy(system_token, sizeof(system_token), word);
                assigned = 1;
            } else if (assigned == 1) {
                uppercase_copy(class_token, sizeof(class_token), word);
                assigned = 2;
            }
        }
    }

    if (system_token[0] == '\0') {
        identity_ux_fail(ux, generation, IDENTITY_UX_ERR_INVALID);
        return true;
    }

    identity_ux_normalize_fields(system_token, class_token, target_token,
                                 slot_token, ux->product_line, ux->device_line,
                                 ux->slot_line);
    if (ux->product_line[0] == '\0') {
        identity_ux_fail(ux, generation, IDENTITY_UX_ERR_INVALID);
        return true;
    }
    ux->state = IDENTITY_UX_SUCCESS;
    ux->error = IDENTITY_UX_ERR_NONE;
    return true;
}
