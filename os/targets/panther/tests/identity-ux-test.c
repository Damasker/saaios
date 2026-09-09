#define _GNU_SOURCE

#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "../src/identity-ux.c"

static void test_normalize_panther(void) {
    char product[IDENTITY_UX_LINE_CHARS + 1];
    char device[IDENTITY_UX_LINE_CHARS + 1];
    char slot[IDENTITY_UX_LINE_CHARS + 1];
    identity_ux_normalize_fields("SAAIOS", "PHONE", "PANTHER", "A", product,
                                 device, slot);
    assert(strcmp(product, "SAAIOS") == 0);
    assert(strcmp(device, "PIXEL 7 / PANTHER") == 0);
    assert(strcmp(slot, "SLOT A") == 0);
}

static void test_normalize_unknown(void) {
    char product[IDENTITY_UX_LINE_CHARS + 1];
    char device[IDENTITY_UX_LINE_CHARS + 1];
    char slot[IDENTITY_UX_LINE_CHARS + 1];
    identity_ux_normalize_fields("SAAIOS", "PHONE", "!!!", "Z", product, device,
                                 slot);
    assert(strcmp(product, "SAAIOS") == 0);
    assert(strcmp(device, "DEVICE / TARGET UNKNOWN") == 0);
    assert(strcmp(slot, "SLOT UNKNOWN") == 0);
}

static void test_complete_success(void) {
    identity_ux ux;
    identity_ux_reset(&ux);
    assert(identity_ux_begin(&ux, 1000));
    assert(identity_ux_complete_from_output(
        &ux, ux.generation,
        "SAAIOS PHONE\nTARGET panther\nBOOT SLOT a\n"));
    assert(ux.state == IDENTITY_UX_SUCCESS);
    assert(strcmp(ux.product_line, "SAAIOS") == 0);
    assert(strcmp(ux.device_line, "PIXEL 7 / PANTHER") == 0);
    assert(strcmp(ux.slot_line, "SLOT A") == 0);
}

static void test_complete_offline(void) {
    identity_ux ux;
    identity_ux_reset(&ux);
    assert(identity_ux_begin(&ux, 1000));
    assert(identity_ux_complete_from_output(&ux, ux.generation,
                                            "SYSTEM CORE OFFLINE\n"));
    assert(ux.state == IDENTITY_UX_ERROR);
    assert(ux.error == IDENTITY_UX_ERR_OFFLINE);
}

static void test_generation_guard(void) {
    identity_ux ux;
    identity_ux_reset(&ux);
    assert(identity_ux_begin(&ux, 1000));
    uint64_t first = ux.generation;
    identity_ux_fail(&ux, first, IDENTITY_UX_ERR_TIMEOUT);
    assert(ux.state == IDENTITY_UX_ERROR);
    assert(!identity_ux_complete_from_output(
        &ux, first, "SAAIOS PHONE\nTARGET PANTHER\nBOOT SLOT A\n"));
    assert(ux.state == IDENTITY_UX_ERROR);
    assert(ux.error == IDENTITY_UX_ERR_TIMEOUT);
}

static void test_single_flight(void) {
    identity_ux ux;
    identity_ux_reset(&ux);
    assert(identity_ux_begin(&ux, 1000));
    assert(!identity_ux_begin(&ux, 1001));
}

static void test_target_token(void) {
    assert(identity_ux_target_token_ok("PANTHER"));
    assert(identity_ux_target_token_ok("A_B-1"));
    assert(!identity_ux_target_token_ok("panther"));
    assert(!identity_ux_target_token_ok("THISISTOOLONGX"));
    assert(!identity_ux_target_token_ok("BAD!"));
}

int main(void) {
    test_normalize_panther();
    test_normalize_unknown();
    test_complete_success();
    test_complete_offline();
    test_generation_guard();
    test_single_flight();
    test_target_token();
    puts("identity-ux-test: ok");
    return 0;
}
