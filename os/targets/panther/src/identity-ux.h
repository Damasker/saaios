#ifndef SAAIOS_IDENTITY_UX_H
#define SAAIOS_IDENTITY_UX_H

#include <stdbool.h>
#include <stdint.h>

#define IDENTITY_UX_TIMEOUT_MS 10000U
#define IDENTITY_UX_LINE_CHARS 24

typedef enum {
    IDENTITY_UX_IDLE = 0,
    IDENTITY_UX_RUNNING,
    IDENTITY_UX_SUCCESS,
    IDENTITY_UX_ERROR
} identity_ux_state;

typedef enum {
    IDENTITY_UX_ERR_NONE = 0,
    IDENTITY_UX_ERR_OFFLINE,
    IDENTITY_UX_ERR_TOOL_MISSING,
    IDENTITY_UX_ERR_TIMEOUT,
    IDENTITY_UX_ERR_INVALID
} identity_ux_error;

typedef struct {
    identity_ux_state state;
    identity_ux_error error;
    uint64_t generation;
    uint64_t started_ms;
    char product_line[IDENTITY_UX_LINE_CHARS + 1];
    char device_line[IDENTITY_UX_LINE_CHARS + 1];
    char slot_line[IDENTITY_UX_LINE_CHARS + 1];
} identity_ux;

void identity_ux_reset(identity_ux *ux);

/* Returns false if an attempt is already running. */
bool identity_ux_begin(identity_ux *ux, uint64_t now_ms);

bool identity_ux_matches(const identity_ux *ux, uint64_t generation);

void identity_ux_fail(identity_ux *ux, uint64_t generation,
                      identity_ux_error error);

/* Parse saaios-console --identity stdout/stderr. Returns false if generation
 * does not match or ux is not running. */
bool identity_ux_complete_from_output(identity_ux *ux, uint64_t generation,
                                      const char *raw_text);

/* Pure helpers exposed for host tests. */
bool identity_ux_target_token_ok(const char *token);
void identity_ux_normalize_fields(const char *system_token,
                                  const char *class_token,
                                  const char *target_token,
                                  const char *slot_token,
                                  char *product_line,
                                  char *device_line,
                                  char *slot_line);

#endif
