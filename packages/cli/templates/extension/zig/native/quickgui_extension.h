/* QuickGUI extension ABI v1. No renderer, host, or C++ runtime dependency.
 * This header may be vendored into an independently built extension package.
 * Keep layouts in sync with crates/quickgui-extension-sdk/src/abi.rs. */
#ifndef QUICKGUI_EXTENSION_H
#define QUICKGUI_EXTENSION_H

#include <stddef.h>
#include <stdint.h>

#ifdef _WIN32
#define QUICKGUI_EXTENSION_EXPORT __declspec(dllexport)
#else
#define QUICKGUI_EXTENSION_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define QUICKGUI_EXTENSION_ABI_V1 1
#define QUICKGUI_EXTENSION_SERVICE 2
#define QUICKGUI_EXTENSION_COMPONENT 3
#define QUICKGUI_EXTENSION_PACKAGE 4
#define QUICKGUI_COMPONENT_MAX_PAYLOAD (16 * 1024 * 1024)
#define QUICKGUI_EXTENSION_MAX_PAYLOAD (64 * 1024)
#define QUICKGUI_EXTENSION_REPLY 0
#define QUICKGUI_EXTENSION_ERROR 1
#define QUICKGUI_EXTENSION_EVENT 2

typedef struct QuickGuiBytes {
    const uint8_t *data;
    size_t len;
} QuickGuiBytes;

/* Ownership transfers to invoke, including error paths. emit borrows its bytes
 * only for the call. release must be called exactly once after the last callback.
 * Replies/events contain JSON; errors contain UTF-8 text. */
typedef struct QuickGuiServiceSink {
    void *context;
    void (*emit)(void *context, uint32_t kind, QuickGuiBytes payload);
    void (*release)(void *context);
} QuickGuiServiceSink;

/* Copy any borrowed inputs retained after returning. Enqueue slow work; never block on UI,
 * network, or worker completion. shutdown cancels sessions without waiting.
 * invoke runs on the frontend UI worker; shutdown may run on the native main thread
 * concurrently with a final invoke. Serialize extension state without waiting on that
 * thread. Workers may emit/release from any thread.
 * A start request's ID identifies its session and its retained event sink. */
typedef struct QuickGuiServiceApi {
    void (*invoke)(uint32_t request, QuickGuiBytes method, QuickGuiBytes params,
                   QuickGuiServiceSink sink);
    void (*shutdown)(void);
} QuickGuiServiceApi;

/* Component instances transfer ownership of wake at create, including failure paths. Calls on
 * each instance run serially on the UI thread. Request/reply bytes are borrowed only during the
 * call, and a reply is made at most once, synchronously. Rendered nodes use the SDK's generic
 * primitive schema; component names, props, state, and behavior belong to the package. */
typedef struct QuickGuiWake {
    void *context;
    void (*wake)(void *context);
    void (*release)(void *context);
} QuickGuiWake;
typedef void (*QuickGuiReply)(void *context, QuickGuiBytes payload);
typedef struct QuickGuiComponentApi {
    void *(*create)(QuickGuiBytes component, QuickGuiBytes props, QuickGuiWake wake,
                    void *context, QuickGuiReply reply);
    int32_t (*update)(void *instance, QuickGuiBytes props);
    int32_t (*render)(void *instance, QuickGuiBytes request, void *context, QuickGuiReply reply);
    int32_t (*event)(void *instance, QuickGuiBytes event, void *context, QuickGuiReply reply);
    void (*destroy)(void *instance);
} QuickGuiComponentApi;

/* A package may expose components and async services under the same name. */
typedef struct QuickGuiPackageApi {
    QuickGuiComponentApi component;
    QuickGuiServiceApi service;
} QuickGuiPackageApi;

/* All fields and the pointed-to table remain valid until process exit.
 * version is the extension's own exact release, not the core's release. */
typedef struct QuickGuiExtension {
    uint32_t abi_version;
    uint32_t descriptor_size;
    uint32_t kind;
    uint32_t api_size;
    QuickGuiBytes name;
    QuickGuiBytes version;
    const void *api;
} QuickGuiExtension;

QUICKGUI_EXTENSION_EXPORT const QuickGuiExtension *quickgui_extension_v1(void);

#ifdef __cplusplus
}
#endif
#endif
