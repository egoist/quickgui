/* An independently compiled component that only includes the public C SDK. */
#include "quickgui_extension.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct Counter { QuickGuiWake wake; unsigned count; } Counter;
static QuickGuiBytes bytes(const char *value) {
    return (QuickGuiBytes){(const uint8_t *)value, strlen(value)};
}
static void *create(QuickGuiBytes name, QuickGuiBytes props, QuickGuiWake wake,
                    void *context, QuickGuiReply reply) {
    (void)props;
    if (name.len != 7 || memcmp(name.data, "counter", 7) != 0) {
        wake.release(wake.context);
        reply(context, bytes("unknown component"));
        return NULL;
    }
    Counter *counter = calloc(1, sizeof(*counter));
    if (!counter) { wake.release(wake.context); return NULL; }
    counter->wake = wake;
    return counter;
}
static int32_t update(void *instance, QuickGuiBytes props) {
    (void)instance; (void)props;
    return 0;
}
static int32_t render(void *instance, QuickGuiBytes request, void *context, QuickGuiReply reply) {
    (void)request;
    Counter *counter = instance;
    char frame[1024];
    snprintf(frame, sizeof(frame),
        "{\"nodes\":[{\"key\":0,\"kind\":\"button\",\"events\":[\"click\"],"
        "\"styles\":[{\"op\":\"width\",\"value\":180},{\"op\":\"height\",\"value\":40}],"
        "\"children\":[{\"key\":1,\"kind\":\"text\",\"text\":\"Count: %u\"}]}]}", counter->count);
    reply(context, bytes(frame));
    return 0;
}
static int32_t event(void *instance, QuickGuiBytes input, void *context, QuickGuiReply reply) {
    (void)input;
    Counter *counter = instance;
    counter->count++;
    reply(context, bytes("{\"changed\":true,\"prevent_default\":false,\"stop_propagation\":false,\"read_clipboard\":false,\"events\":[]}"));
    return 0;
}
static void destroy(void *instance) {
    Counter *counter = instance;
    counter->wake.release(counter->wake.context);
    free(counter);
}
static const QuickGuiComponentApi api = {create, update, render, event, destroy};
static const QuickGuiExtension extension = {
    QUICKGUI_EXTENSION_ABI_V1, sizeof(QuickGuiExtension), QUICKGUI_EXTENSION_COMPONENT,
    sizeof(QuickGuiComponentApi), {(const uint8_t *)"acme-counter", 12},
    {(const uint8_t *)"7.2.1", 5}, &api
};
QUICKGUI_EXTENSION_EXPORT const QuickGuiExtension *quickgui_extension_v1(void) { return &extension; }
