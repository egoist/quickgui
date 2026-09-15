//! Versioned, process-local extension ABI. Only C layouts, borrowed spans and opaque handles
//! cross this boundary. Extensions must not link another copy of the renderer or host runtime.

use std::ffi::c_void;

pub const ABI_VERSION: u32 = 1;
/// Generic request/reply/event extension. Names and release versions belong to
/// the extension author; no per-extension kind or core registration is needed.
pub const SERVICE_EXTENSION: u32 = 2;
/// A package-owned retained component, rendered using the host's ordinary primitives.
pub const COMPONENT_EXTENSION: u32 = 3;
/// One independently versioned package providing components and asynchronous services.
pub const PACKAGE_EXTENSION: u32 = 4;
pub const MAX_EXTENSION_NAME: usize = 64;
pub const MAX_EXTENSIONS: usize = 32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bytes {
    pub data: *const u8,
    pub len: usize,
}

impl Bytes {
    pub const fn new(value: &[u8]) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len(),
        }
    }
}

/// The descriptor and its function table remain valid until process exit. A consumer checks
/// the header before reading the table. The release version matches the importing
/// Go package's requirement, independently of the core version for generic services.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Extension {
    pub abi_version: u32,
    pub descriptor_size: u32,
    pub kind: u32,
    pub api_size: u32,
    pub name: Bytes,
    pub version: Bytes,
    pub api: *const c_void,
}

// SAFETY: Published descriptors only refer to immutable static bytes and function tables.
unsafe impl Sync for Extension {}

/// Ownership of the callback context transfers to `create`, including its error path. The
/// extension calls release exactly once after its final worker can no longer call wake.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Wake {
    pub context: *mut c_void,
    pub wake: unsafe extern "C" fn(*mut c_void),
    pub release: unsafe extern "C" fn(*mut c_void),
}

pub type Reply = unsafe extern "C" fn(*mut c_void, Bytes);

/// A service owns this context until its last callback, including rejection paths. Kind 0 is
/// one successful JSON reply, kind 1 a UTF-8 error, and kind 2 a JSON session event. Payloads
/// are bounded to 64 KiB and borrowed only during emit. Events may follow the start reply until
/// stop; release is called exactly once, after the final callback on any worker thread.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ServiceSink {
    pub context: *mut c_void,
    pub emit: unsafe extern "C" fn(*mut c_void, u32, Bytes),
    pub release: unsafe extern "C" fn(*mut c_void),
}

/// Calls enqueue work and return immediately. Request IDs identify sessions started with
/// "start"; subsequent commands carry that ID in their JSON. Shutdown cancels sessions without
/// waiting for network or main-thread work. Copy any borrowed input retained after returning.
/// One-shot operations can reply and release their sink without opening a session.
/// Invoke runs on the frontend UI worker; shutdown may run on the native main thread
/// concurrently with a final invoke. Extensions must serialize their own state without
/// blocking the main thread.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ServiceApi {
    pub invoke: unsafe extern "C" fn(u32, Bytes, Bytes, ServiceSink),
    pub shutdown: unsafe extern "C" fn(),
}

/// Retained component lifecycle. All calls for one instance run serially on the UI thread.
/// `create` takes ownership of `wake`, including on failure. Instances release it after their
/// final worker stops. All other spans are borrowed for the duration of the call. `render` and
/// `event` invoke their reply at most once, synchronously; replies use the component schema.
/// No Rust values, renderer objects, or allocator ownership cross this boundary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ComponentApi {
    pub create: unsafe extern "C" fn(Bytes, Bytes, Wake, *mut c_void, Reply) -> *mut c_void,
    pub update: unsafe extern "C" fn(*mut c_void, Bytes) -> i32,
    pub render: unsafe extern "C" fn(*mut c_void, Bytes, *mut c_void, Reply) -> i32,
    pub event: unsafe extern "C" fn(*mut c_void, Bytes, *mut c_void, Reply) -> i32,
    pub destroy: unsafe extern "C" fn(*mut c_void),
}

/// Both capabilities share the descriptor's name and lifetime. Neither table may be absent.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PackageApi {
    pub component: ComponentApi,
    pub service: ServiceApi,
}
