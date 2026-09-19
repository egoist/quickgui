//! Native extension wrapper around QuickGUI's updater. The implementation is the core crate's
//! `updater` module, compiled here without the renderer or application runtime; this crate adds
//! only the bounded service ABI that Go and TypeScript applications load.

// The Rust client API in these sources is unused here; the core crate checks them for dead code.
#[allow(dead_code, unused_imports)]
#[path = "../../../src/updater/mod.rs"]
mod updater;
#[cfg(not(target_os = "macos"))]
pub use updater::handoff;

use quickgui_extension_sdk::abi::{self, Bytes, ServiceSink};
use std::sync::Arc;
use updater::Sink;

unsafe fn copy_bytes<'a>(bytes: Bytes, maximum: usize) -> Result<&'a str, String> {
    if bytes.len > maximum || bytes.data.is_null() {
        return Err("invalid updater request span".into());
    }
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) })
        .map_err(|_| "updater request must be UTF-8".into())
}
unsafe extern "C" fn invoke(request: u32, method: Bytes, params: Bytes, callback: ServiceSink) {
    let sink = Arc::new(Sink(callback));
    let input = unsafe {
        copy_bytes(method, 64)
            .and_then(|m| copy_bytes(params, 64 * 1024).map(|p| (m.to_owned(), p.to_owned())))
    };
    match input {
        Ok((method, params)) => updater::dispatch(request, method, params, sink),
        Err(e) => sink.error(e),
    }
}
unsafe extern "C" fn shutdown() {
    updater::shutdown();
}

static API: abi::ServiceApi = abi::ServiceApi { invoke, shutdown };
static EXTENSION: abi::Extension = abi::Extension {
    abi_version: abi::ABI_VERSION,
    descriptor_size: size_of::<abi::Extension>() as u32,
    kind: abi::SERVICE_EXTENSION,
    api_size: size_of::<abi::ServiceApi>() as u32,
    name: Bytes::new(b"updater"),
    version: Bytes::new(env!("CARGO_PKG_VERSION").as_bytes()),
    api: std::ptr::addr_of!(API).cast(),
};
/// # Safety
/// The returned descriptor and function table must only be used according to the extension ABI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_extension_v1() -> *const abi::Extension {
    &EXTENSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Default)]
    struct Capture {
        messages: Mutex<Vec<(u32, Vec<u8>)>>,
        releases: AtomicUsize,
    }
    unsafe extern "C" fn record(context: *mut c_void, kind: u32, bytes: Bytes) {
        let capture = unsafe { &*context.cast::<Arc<Capture>>() };
        capture.messages.lock().unwrap().push((
            kind,
            unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) }.to_vec(),
        ));
    }
    unsafe extern "C" fn release(context: *mut c_void) {
        let capture = unsafe { Box::from_raw(context.cast::<Arc<Capture>>()) };
        capture.releases.fetch_add(1, Ordering::Relaxed);
    }
    #[test]
    fn malformed_abi_input_releases_context_even_before_dispatch() {
        let capture = Arc::new(Capture::default());
        let callback = ServiceSink {
            context: Box::into_raw(Box::new(capture.clone())).cast(),
            emit: record,
            release,
        };
        unsafe {
            invoke(
                1,
                Bytes {
                    data: std::ptr::null(),
                    len: 1,
                },
                Bytes::new(b"{}"),
                callback,
            )
        };
        assert_eq!(capture.releases.load(Ordering::Relaxed), 1);
        assert_eq!(capture.messages.lock().unwrap()[0].0, 1);
    }
}
