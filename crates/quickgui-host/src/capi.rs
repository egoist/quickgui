//! The shared-library C ABI loaded by the purego frontend.
//!
//! Every function that reaches the platform application thread is fire-and-forget: it enqueues a
//! bounded host command and returns immediately. Results, lifecycle outcomes, and request
//! acceptance return later through the registered [`EventCallback`], so the application thread
//! never waits on native main-thread execution. CPU-only services answer synchronously through a
//! call-scoped reply callback because they never touch the application runtime.

use std::ffi::{CStr, c_char, c_int, c_void};

use super::*;

/// Borrow one length-delimited span passed across the boundary. An empty span may be null.
///
/// # Safety
/// `pointer` must be valid for `length` bytes for the duration of the call, or `length` must be 0.
unsafe fn span<'a>(pointer: *const u8, length: usize) -> &'a [u8] {
    if pointer.is_null() || length == 0 {
        &[]
    } else {
        // SAFETY: the caller guarantees the span is readable for `length` bytes.
        unsafe { std::slice::from_raw_parts(pointer, length) }
    }
}

/// # Safety
/// See [`span`].
unsafe fn text<'a>(pointer: *const u8, length: usize) -> std::result::Result<&'a str, String> {
    // SAFETY: forwarded to `span` under the same contract.
    std::str::from_utf8(unsafe { span(pointer, length) })
        .map_err(|_| "QuickGUI strings must contain valid UTF-8".to_owned())
}

fn parse_options<T: for<'de> Deserialize<'de> + Default>(
    json: &str,
    what: &str,
) -> std::result::Result<T, String> {
    if json.trim().is_empty() {
        return Ok(T::default());
    }
    serde_json::from_str(json).map_err(|error| format!("invalid {what}: {error}"))
}

/// Report a caller error: the application misused the boundary, which is fatal for the host.
fn fatal(error: String) -> c_int {
    eprintln!("quickgui: {error}");
    HOST.fail(error);
    -1
}

fn enqueue(command: HostCommand) -> c_int {
    match HOST.enqueue(command) {
        Ok(()) => 0,
        Err(error) => fatal(error),
    }
}

#[cfg(target_os = "macos")]
struct HostAutoreleasePool {
    context: *mut core::ffi::c_void,
}

#[cfg(target_os = "macos")]
impl HostAutoreleasePool {
    fn new() -> Self {
        // SAFETY: the host loop owns the process main thread until the native loop exits. No pool
        // created by the application can be popped across this one while that call is active.
        let context = unsafe { objc2::ffi::objc_autoreleasePoolPush() };
        Self { context }
    }

    fn drain_and_replace(&mut self) {
        // AppKit can autorelease an NSWindow or its delegate after an inner Winit pool has already
        // drained. Rotate the host-level pool after each pump so those final references never fall
        // through to the process-lifetime pool.
        unsafe {
            objc2::ffi::objc_autoreleasePoolPop(self.context);
            self.context = objc2::ffi::objc_autoreleasePoolPush();
        }
    }
}

#[cfg(target_os = "macos")]
impl Drop for HostAutoreleasePool {
    fn drop(&mut self) {
        // SAFETY: The host loop is entered and left on the same process main thread, and all Winit
        // callback-local pools have drained before the loop returns.
        unsafe {
            objc2::ffi::objc_autoreleasePoolPop(self.context);
        }
    }
}

/// Run AppKit/Winit on the process main thread, locked by Go's runtime.LockOSThread.
/// The Go UI goroutine independently submits commands through the nonblocking C ABI.
#[unsafe(no_mangle)]
pub extern "C" fn quickgui_run_host() -> c_int {
    run_host()
}

/// Run the native host loop on the current (main) thread until the application exits.
///
/// Returns the process exit code. A host failure prints its reason and exits with status 1.
/// The core runtime announces the first ready turn on `QUICKGUI_READY_SOCKET` for development.
pub fn run_host() -> i32 {
    if let Err(error) = HOST.begin() {
        eprintln!("quickgui: {error}");
        return 1;
    }
    let result = run_app_host_loop();
    let code = match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("quickgui: {error}");
            HOST.fail(error);
            1
        }
    };
    HOST.finish();
    code
}

#[unsafe(no_mangle)]
pub extern "C" fn quickgui_protocol_version() -> u32 {
    PROTOCOL_VERSION as u32
}

/// Register an optional native extension before creating application windows.
///
/// # Safety
/// The descriptor and its code must remain loaded until process exit. `name` is a readable
/// UTF-8 span identifying the extension requested by the importing Go package.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_register_extension(
    descriptor: *const quickgui::extension_api::Extension,
    name: *const u8,
    length: usize,
) -> c_int {
    let result = (|| {
        if descriptor.is_null() || length > quickgui::extension_api::MAX_EXTENSION_NAME {
            return Err("invalid extension registration".to_owned());
        }
        let expected = unsafe { text(name, length) }?;
        unsafe { quickgui::extensions::register_extension(descriptor, expected.as_bytes()) }
            .map_err(str::to_owned)
    })();
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("quickgui: {error}");
            -1
        }
    }
}

/// Register an extension whose release version is owned by its importing package.
///
/// # Safety
/// The descriptor follows `quickgui_register_extension`'s lifetime contract.
/// Both input spans must be readable for their declared lengths during this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_register_extension_versioned(
    descriptor: *const quickgui::extension_api::Extension,
    name: *const u8,
    name_length: usize,
    version: *const u8,
    version_length: usize,
) -> c_int {
    let result = (|| {
        if descriptor.is_null()
            || name_length > quickgui::extension_api::MAX_EXTENSION_NAME
            || version_length == 0
            || version_length > 64
        {
            return Err("invalid extension registration".to_owned());
        }
        let name = unsafe { text(name, name_length) }?;
        let version = unsafe { text(version, version_length) }?;
        unsafe {
            quickgui::extensions::register_extension_versioned(
                descriptor,
                name.as_bytes(),
                version.as_bytes(),
            )
        }
        .map_err(str::to_owned)
    })();
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("quickgui: {error}");
            -1
        }
    }
}

/// Register the application thread's event callback. Events published earlier are flushed to it.
///
/// # Safety
/// `callback` must remain callable from any thread until [`quickgui_clear_event_callback`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_set_event_callback(
    callback: EventCallback,
    context: *mut c_void,
) {
    HOST.set_sink(Some(EventSink {
        callback,
        context: context as usize,
    }));
}

/// # Safety
/// Pairs with [`quickgui_set_event_callback`]; the pointers are not dereferenced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_clear_event_callback(
    _callback: EventCallback,
    _context: *mut c_void,
) {
    HOST.set_sink(None);
}

/// Allocate the application handle and queue its creation. Returns 0 on a fatal error.
///
/// # Safety
/// `options` is a readable JSON span of `length` bytes, or empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_create_app(options: *const u8, length: usize) -> u32 {
    // SAFETY: forwarded under the caller contract.
    let json = match unsafe { text(options, length) } {
        Ok(json) => json,
        Err(error) => {
            fatal(error);
            return 0;
        }
    };
    let options: NativeAppOptions = match parse_options(json, "application options") {
        Ok(options) => options,
        Err(error) => {
            fatal(error);
            return 0;
        }
    };
    let app = match HOST
        .allocate_app()
        .and_then(|app| HOST.set_app(app).map(|()| app))
    {
        Ok(app) => app,
        Err(error) => {
            fatal(error);
            return 0;
        }
    };
    if enqueue(HostCommand::CreateApp { app, options }) != 0 {
        return 0;
    }
    app
}

#[unsafe(no_mangle)]
pub extern "C" fn quickgui_prepare_app(app: u32, request: u32) -> c_int {
    enqueue(HostCommand::PrepareApp { app, request })
}

#[unsafe(no_mangle)]
pub extern "C" fn quickgui_allocate_window() -> u32 {
    match HOST.allocate_window() {
        Ok(window) => window,
        Err(error) => {
            fatal(error);
            0
        }
    }
}

/// # Safety
/// `options` and `batch` are readable spans, or empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_create_window(
    app: u32,
    window: u32,
    options: *const u8,
    options_length: usize,
    batch: *const u8,
    batch_length: usize,
) -> c_int {
    // SAFETY: forwarded under the caller contract.
    let options = match unsafe { text(options, options_length) }
        .and_then(|json| parse_options::<NativeWindowOptions>(json, "window options"))
    {
        Ok(options) => options,
        Err(error) => return fatal(error),
    };
    // SAFETY: forwarded under the caller contract.
    let initial_batch = unsafe { span(batch, batch_length) }.to_vec();
    enqueue(HostCommand::CreateWindow {
        app,
        window,
        options,
        initial_batch,
    })
}

/// # Safety
/// `options` and `batch` are readable spans, or empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_create_system_popover(
    app: u32,
    window: u32,
    parent: u32,
    anchor: u32,
    options: *const u8,
    options_length: usize,
    batch: *const u8,
    batch_length: usize,
) -> c_int {
    // SAFETY: forwarded under the caller contract.
    let options = match unsafe { text(options, options_length) }
        .and_then(|json| parse_options::<NativeWindowOptions>(json, "popover options"))
    {
        Ok(options) => options,
        Err(error) => return fatal(error),
    };
    // SAFETY: forwarded under the caller contract.
    let initial_batch = unsafe { span(batch, batch_length) }.to_vec();
    enqueue(HostCommand::CreateSystemPopover {
        app,
        window,
        parent,
        anchor,
        options,
        initial_batch,
    })
}

/// # Safety
/// `options` and `batch` are readable spans, or empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_create_embedded_view(
    app: u32,
    window: u32,
    parent: u32,
    match_horizontal: u8,
    match_vertical: u8,
    options: *const u8,
    options_length: usize,
    batch: *const u8,
    batch_length: usize,
) -> c_int {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: forwarded under the caller contract.
        let options = match unsafe { text(options, options_length) }
            .and_then(|json| parse_options::<NativeWindowOptions>(json, "embedded view options"))
        {
            Ok(options) => options,
            Err(error) => return fatal(error),
        };
        // SAFETY: forwarded under the caller contract.
        let initial_batch = unsafe { span(batch, batch_length) }.to_vec();
        enqueue(HostCommand::CreateEmbeddedView {
            app,
            window,
            parent,
            match_horizontal: match_horizontal != 0,
            match_vertical: match_vertical != 0,
            options,
            initial_batch,
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (
            app,
            window,
            parent,
            match_horizontal,
            match_vertical,
            options,
            options_length,
            batch,
            batch_length,
        );
        fatal("embedded SwiftUI QuickGUI views require macOS".to_owned())
    }
}

/// # Safety
/// `batch` is a readable span of `length` bytes, or empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_apply_batch(
    app: u32,
    window: u32,
    batch: *const u8,
    length: usize,
) -> c_int {
    if length > MAX_BATCH_BYTES {
        return fatal(format!("mutation batch exceeds {MAX_BATCH_BYTES} bytes"));
    }
    // SAFETY: forwarded under the caller contract.
    let batch = unsafe { span(batch, length) }.to_vec();
    enqueue(HostCommand::ApplyBatch { app, window, batch })
}

#[unsafe(no_mangle)]
pub extern "C" fn quickgui_close_window(app: u32, window: u32) -> c_int {
    enqueue(HostCommand::CloseWindow { app, window })
}

#[unsafe(no_mangle)]
pub extern "C" fn quickgui_focus_node(app: u32, window: u32, node: u32) -> c_int {
    enqueue(HostCommand::FocusNode { app, window, node })
}

/// Present a native dialog: `kind` 0 is an alert, 1 an open panel, 2 a save panel. `window` 0
/// presents application-modally. The outcome returns as an `alert-dialog`, `open-dialog`, or
/// `save-dialog` event carrying `request`.
///
/// # Safety
/// `options` is a readable JSON span of `length` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_show_dialog(
    app: u32,
    window: u32,
    request: u32,
    kind: u32,
    options: *const u8,
    length: usize,
) -> c_int {
    // SAFETY: forwarded under the caller contract.
    let json = match unsafe { text(options, length) } {
        Ok(json) => json,
        Err(error) => return fatal(error),
    };
    let window = (window != 0).then_some(window);
    match kind {
        0 => match serde_json::from_str::<NativeDialogOptions>(json) {
            Ok(options) => enqueue(HostCommand::ShowAlertDialog {
                app,
                window,
                request,
                options,
            }),
            Err(error) => fatal(format!("invalid alert dialog options: {error}")),
        },
        1 => match serde_json::from_str::<NativeOpenDialogOptions>(json) {
            Ok(options) => enqueue(HostCommand::ShowOpenDialog {
                app,
                window,
                request,
                options,
            }),
            Err(error) => fatal(format!("invalid open dialog options: {error}")),
        },
        2 => match serde_json::from_str::<NativeSaveDialogOptions>(json) {
            Ok(options) => enqueue(HostCommand::ShowSaveDialog {
                app,
                window,
                request,
                options,
            }),
            Err(error) => fatal(format!("invalid save dialog options: {error}")),
        },
        kind => fatal(format!("unknown native dialog kind {kind}")),
    }
}

/// Queue one JSON-encoded system command. A zero `request` is a fire-and-forget mutation; any
/// other value returns the result as one `command` event carrying that request id.
///
/// # Safety
/// `json` is a readable span of `length` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_command(
    app: u32,
    request: u32,
    json: *const u8,
    length: usize,
) -> c_int {
    // SAFETY: forwarded under the caller contract.
    let json = match unsafe { text(json, length) } {
        Ok(json) => json,
        Err(error) => return fatal(error),
    };
    let command = match parse_system_request(json) {
        Ok(command) => command,
        Err(error) => {
            if request == 0 {
                return fatal(error);
            }
            publish_reply("command", request, Err(error));
            return 0;
        }
    };
    if request == 0 {
        enqueue(HostCommand::Mutation { app, command })
    } else {
        enqueue(HostCommand::Request {
            app,
            request,
            command,
        })
    }
}

/// Run one background integration (autostart, secure storage, updater, crash reports, metrics)
/// off the application thread; the result returns as one `invoke` event carrying `request`.
///
/// # Safety
/// `method` and `params` are readable spans.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_invoke(
    request: u32,
    method: *const u8,
    method_length: usize,
    params: *const u8,
    params_length: usize,
) -> c_int {
    // SAFETY: forwarded under the caller contract.
    let (method, params) = match (unsafe { text(method, method_length) }, unsafe {
        text(params, params_length)
    }) {
        (Ok(method), Ok(params)) => (method.to_owned(), params.to_owned()),
        (Err(error), _) | (_, Err(error)) => return fatal(error),
    };
    if let Some(service) = method.strip_prefix("extension/") {
        crate::extension_services::invoke(request, service, &params);
        return 0;
    }
    let spawned = std::thread::Builder::new()
        .name("quickgui-invoke".to_owned())
        .spawn(move || {
            let result = integrations::invoke(&method, &params, request);
            publish_reply("invoke", request, result);
        });
    match spawned {
        Ok(_) => 0,
        Err(error) => {
            publish_reply(
                "invoke",
                request,
                Err(format!("could not start the integration thread: {error}")),
            );
            0
        }
    }
}

/// Answer one CPU-only service synchronously. The reply callback receives one JSON document of
/// the form `{"ok":true,"value":...}` or `{"ok":false,"error":"..."}` before this call returns.
///
/// # Safety
/// `method` and `params` are readable spans; `reply` is called exactly once during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_call(
    method: *const u8,
    method_length: usize,
    params: *const u8,
    params_length: usize,
    reply: unsafe extern "C" fn(*const u8, usize, *mut c_void),
    context: *mut c_void,
) -> c_int {
    // SAFETY: forwarded under the caller contract.
    let result = match (unsafe { text(method, method_length) }, unsafe {
        text(params, params_length)
    }) {
        (Ok(method), Ok(params)) => integrations::call(method, params),
        (Err(error), _) | (_, Err(error)) => Err(error),
    };
    let document = match result {
        Ok(mut value) => {
            strip_nulls(&mut value);
            serde_json::json!({ "ok": true, "value": value })
        }
        Err(error) => serde_json::json!({ "ok": false, "error": error }),
    }
    .to_string();
    // SAFETY: the reply callback copies the span before returning.
    unsafe { reply(document.as_ptr(), document.len(), context) };
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn quickgui_destroy_app(app: u32) -> c_int {
    enqueue(HostCommand::DestroyApp { app })
}

/// Report an application-side failure while the main thread is blocked in the native loop.
///
/// # Safety
/// `message` is a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_abort(message: *const c_char) {
    let message = if message.is_null() {
        "the QuickGUI application failed".to_owned()
    } else {
        // SAFETY: the caller passes a NUL-terminated string.
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned()
    };
    HOST.fail(message);
}

pub(super) fn run_app_host_loop() -> std::result::Result<i32, String> {
    #[cfg(target_os = "macos")]
    let mut autorelease_pool = HostAutoreleasePool::new();
    let mut active_app = None;
    let mut runtime: Option<NativeRuntime> = None;
    // Window handles are allocated before the platform mounts them during a native turn.
    // A separate creation notification also covers hidden windows, which may never present.
    let mut pending_windows = Vec::new();
    // Release services on normal exit, explicit destruction, and error paths, while AppKit's
    // main-thread autorelease pool and runtime are still alive.
    let _services = crate::extension_services::ApplicationServices;

    loop {
        let running = runtime
            .as_ref()
            .is_some_and(|runtime| runtime.runner.is_some());
        let mut commands = if running {
            HOST.take_commands()?
        } else {
            HOST.wait_for_commands()?
        };

        while let Some(command) = commands.pop_front() {
            match command {
                HostCommand::CreateApp { app, options } => {
                    if runtime.is_some() {
                        return Err("a QuickGUI native host can own only one app".to_owned());
                    }
                    runtime = Some(NativeRuntime::new(options)?);
                    active_app = Some(app);
                }
                HostCommand::CreateWindow {
                    app,
                    window,
                    options,
                    initial_batch,
                } => {
                    with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                        runtime
                            .create_window_with_id(window, options, &initial_batch)
                            .map(|_| ())
                    })?;
                    pending_windows.retain(|id| {
                        runtime
                            .as_ref()
                            .is_some_and(|runtime| runtime.windows.contains_key(id))
                    });
                    pending_windows.push(window);
                }
                HostCommand::CreateSystemPopover {
                    app,
                    window,
                    parent,
                    anchor,
                    options,
                    initial_batch,
                } => {
                    with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                        runtime
                            .create_system_popover_with_id(
                                window,
                                parent,
                                anchor,
                                options,
                                &initial_batch,
                            )
                            .map(|_| ())
                    })?;
                    pending_windows.push(window);
                }
                #[cfg(target_os = "macos")]
                HostCommand::CreateEmbeddedView {
                    app,
                    window,
                    parent,
                    match_horizontal,
                    match_vertical,
                    options,
                    initial_batch,
                } => {
                    with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                        runtime
                            .create_embedded_view_with_id(
                                window,
                                parent,
                                match_horizontal,
                                match_vertical,
                                options,
                                &initial_batch,
                            )
                            .map(|_| ())
                    })?;
                    pending_windows.push(window);
                }
                HostCommand::ApplyBatch { app, window, batch } => {
                    with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                        runtime.apply_batch(window, &batch)
                    })?;
                }
                HostCommand::Mutation { app, command } => {
                    let result =
                        with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                            runtime.execute_system_command(command)
                        })?;
                    if !matches!(result, system::SystemCommandResult::Unit) {
                        return Err("a hosted mutation returned a value".to_owned());
                    }
                }
                HostCommand::Request {
                    app,
                    request,
                    command,
                } => {
                    let result =
                        with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                            runtime.execute_system_command(command)
                        })
                        .map(system_result_json);
                    publish_reply("command", request, result);
                }
                HostCommand::CloseWindow { app, window } => {
                    pending_windows.retain(|id| *id != window);
                    with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                        runtime.close_window(window);
                        Ok(())
                    })?;
                }
                HostCommand::FocusNode { app, window, node } => {
                    with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                        runtime.focus_node(window, node).map(|_| ())
                    })?;
                }
                HostCommand::ShowAlertDialog {
                    app,
                    window,
                    request,
                    options,
                } => {
                    let result =
                        with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                            runtime.show_alert_dialog(window, request, options)
                        });
                    if let Err(error) = result {
                        HOST.publish_events([NativeEvent::reply(
                            "alert-dialog",
                            request,
                            None,
                            Some(error),
                        )]);
                    }
                }
                HostCommand::ShowOpenDialog {
                    app,
                    window,
                    request,
                    options,
                } => {
                    let result =
                        with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                            runtime.show_open_dialog(window, request, options)
                        });
                    if let Err(error) = result {
                        HOST.publish_events([NativeEvent::reply(
                            "open-dialog",
                            request,
                            None,
                            Some(error),
                        )]);
                    }
                }
                HostCommand::ShowSaveDialog {
                    app,
                    window,
                    request,
                    options,
                } => {
                    let result =
                        with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                            runtime.show_save_dialog(window, request, options)
                        });
                    if let Err(error) = result {
                        HOST.publish_events([NativeEvent::reply(
                            "save-dialog",
                            request,
                            None,
                            Some(error),
                        )]);
                    }
                }
                HostCommand::PrepareApp { app, request } => {
                    let result = with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        NativeRuntime::prepare,
                    );
                    if result.is_ok() {
                        let waker = runtime
                            .as_ref()
                            .and_then(|runtime| runtime.runner.as_ref())
                            .expect("a started QuickGUI host must own an AppRunner")
                            .waker();
                        HOST.set_waker(waker);
                    }
                    publish_reply(
                        "app-ready",
                        request,
                        result.map(|()| serde_json::Value::Null),
                    );
                }
                HostCommand::DestroyApp { app } => {
                    crate::integrations::clear_file_watchers();
                    crate::runtime::reset_interception_state();
                    if active_app == Some(app) && runtime.is_some() {
                        HOST.publish_exit(0);
                    }
                    return Ok(0);
                }
            }
        }

        let Some(runtime) = runtime.as_mut() else {
            #[cfg(target_os = "macos")]
            autorelease_pool.drain_and_replace();
            continue;
        };
        runtime.sync_closed_windows();
        HOST.publish_events(runtime.drain_events());

        let Some(runner) = runtime.runner.as_mut() else {
            #[cfg(target_os = "macos")]
            autorelease_pool.drain_and_replace();
            continue;
        };
        let status = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runner.pump(None)))
            .map_err(|payload| {
                format!(
                    "QuickGUI event loop panicked: {}",
                    panic_payload_message(payload.as_ref())
                )
            })?
            .map_err(|error| error.to_string())?;
        runtime.sync_closed_windows();
        HOST.publish_events(runtime.drain_events());
        #[cfg(target_os = "macos")]
        autorelease_pool.drain_and_replace();
        pending_windows.retain(|id| {
            let Some(window) = runtime.windows.get(id) else {
                return false;
            };
            let mounted = window
                .handle
                .and_then(|handle| runtime.runner.as_ref()?.window_state(handle))
                .is_some();
            if mounted {
                HOST.publish_events([NativeEvent {
                    kind: "window-created".to_owned(),
                    window: *id,
                    ..NativeEvent::default()
                }]);
            }
            !mounted
        });
        if let AppRunStatus::Exited(code) = status {
            let code = code.max(0);
            HOST.publish_exit(code);
            return Ok(code);
        }
    }
}

pub(super) fn panic_payload_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&'static str>().copied())
        .unwrap_or("unknown panic payload")
}

pub(super) fn with_hosted_runtime<T>(
    active_app: Option<u32>,
    runtime: Option<&mut NativeRuntime>,
    app: u32,
    callback: impl FnOnce(&mut NativeRuntime) -> std::result::Result<T, String>,
) -> std::result::Result<T, String> {
    if active_app != Some(app) {
        return Err(format!("unknown QuickGUI hosted app {app}"));
    }
    callback(runtime.ok_or_else(|| format!("unknown QuickGUI hosted app {app}"))?)
}

pub(super) fn finite_number(value: Option<f64>) -> Option<f32> {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(f32::MIN as f64, f32::MAX as f64) as f32)
}

pub(super) fn finite_dimension(value: Option<f64>, fallback: f32) -> f32 {
    finite_number(value)
        .filter(|value| *value > 0.0)
        .unwrap_or(fallback)
}
