//! Extension registration happens before the host starts. Registered images remain loaded for
//! process lifetime; individual sessions still release all resources when they unmount.
use crate::extension_api::{self as abi, ComponentApi, Extension, ServiceApi};
use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};

#[derive(Clone)]
struct RegisteredService {
    version: String,
    api: ServiceApi,
}

#[derive(Clone)]
struct RegisteredComponent {
    version: String,
    api: ComponentApi,
}

#[derive(Default)]
struct Registry {
    services: BTreeMap<String, RegisteredService>,
    components: BTreeMap<String, RegisteredComponent>,
}

static SERVICES: OnceLock<Mutex<Registry>> = OnceLock::new();

fn services() -> &'static Mutex<Registry> {
    SERVICES.get_or_init(Default::default)
}

pub fn service(name: &str) -> Option<ServiceApi> {
    services()
        .lock()
        .unwrap()
        .services
        .get(name)
        .map(|service| service.api)
}

/// Look up a component package. Package names and component types are extension-owned.
pub fn component(name: &str) -> Option<ComponentApi> {
    services()
        .lock()
        .unwrap()
        .components
        .get(name)
        .map(|entry| entry.api)
}

pub fn shutdown_services() {
    // Never hold a registry lock while calling foreign code. Extensions can emit
    // their last events and release their sinks during shutdown.
    let extensions: Vec<_> = services()
        .lock()
        .unwrap()
        .services
        .values()
        .map(|service| service.api)
        .collect();
    for api in extensions {
        unsafe { (api.shutdown)() };
    }
}

/// Register an extension obtained from `quickgui_extension_v1` through an in-process loader.
///
/// # Safety
/// The descriptor, its borrowed strings, and function table must be readable and remain valid
/// until process exit. All functions must implement the documented ABI and ownership contracts.
pub unsafe fn register_extension(
    pointer: *const Extension,
    expected_name: &[u8],
) -> Result<(), &'static str> {
    unsafe {
        register_extension_versioned(pointer, expected_name, env!("CARGO_PKG_VERSION").as_bytes())
    }
}

/// Register an independently versioned extension. The imported package supplies
/// its exact version; compatibility is checked by the ABI and table layout.
///
/// # Safety
/// The same descriptor, lifetime, and function contracts as `register_extension` apply.
pub unsafe fn register_extension_versioned(
    pointer: *const Extension,
    expected_name: &[u8],
    expected_version: &[u8],
) -> Result<(), &'static str> {
    if pointer.is_null() {
        return Err("extension descriptor is null");
    }
    // Read only the fixed two-word header until the caller's layout is known to match.
    let header = pointer.cast::<u32>();
    if unsafe { header.read() } != abi::ABI_VERSION
        || unsafe { header.add(1).read() } as usize != size_of::<Extension>()
    {
        return Err("extension ABI version or descriptor size mismatch");
    }
    let descriptor = unsafe { &*pointer };
    if descriptor.name.len == 0
        || descriptor.name.len > abi::MAX_EXTENSION_NAME
        || descriptor.version.len == 0
        || descriptor.version.len > 64
        || descriptor.name.data.is_null()
        || descriptor.version.data.is_null()
    {
        return Err("invalid extension metadata");
    }
    let name = unsafe { std::slice::from_raw_parts(descriptor.name.data, descriptor.name.len) };
    if name != expected_name {
        return Err("loaded extension does not match the requested package");
    }
    if name == b"host" || !valid_name(name) {
        return Err("invalid extension name");
    }
    let version =
        unsafe { std::slice::from_raw_parts(descriptor.version.data, descriptor.version.len) };
    if version != expected_version {
        eprintln!(
            "quickgui extension {}: loaded version {:?}, requested {:?}",
            String::from_utf8_lossy(name),
            String::from_utf8_lossy(version),
            String::from_utf8_lossy(expected_version)
        );
        return Err("extension release does not match the importing package");
    }
    if descriptor.kind == abi::COMPONENT_EXTENSION {
        if descriptor.api.is_null()
            || descriptor.api_size as usize != size_of::<ComponentApi>()
            || !unsafe { valid_functions(descriptor.api, 5) }
        {
            return Err("component extension function table mismatch");
        }
        let api = unsafe { *(descriptor.api.cast::<ComponentApi>()) };
        return services().lock().unwrap().register_component(
            std::str::from_utf8(name).map_err(|_| "invalid extension name")?,
            std::str::from_utf8(version).map_err(|_| "invalid extension version")?,
            api,
        );
    }
    if descriptor.kind == abi::SERVICE_EXTENSION {
        if descriptor.api.is_null()
            || descriptor.api_size as usize != size_of::<ServiceApi>()
            || !unsafe { valid_functions(descriptor.api, 2) }
        {
            return Err("service extension function table mismatch");
        }
        let api = unsafe { *(descriptor.api.cast::<ServiceApi>()) };
        return services().lock().unwrap().register(
            std::str::from_utf8(name).map_err(|_| "invalid extension name")?,
            std::str::from_utf8(version).map_err(|_| "invalid extension version")?,
            api,
        );
    }
    Err("extension is not supported by this core version")
}

fn valid_name(name: &[u8]) -> bool {
    name.first().is_some_and(u8::is_ascii_lowercase)
        && name
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

unsafe fn valid_functions(table: *const std::ffi::c_void, count: usize) -> bool {
    // Check null entries before constructing Rust's non-null function-pointer types.
    (0..count).all(|index| unsafe { table.cast::<usize>().add(index).read() != 0 })
}

impl Registry {
    fn register_component(
        &mut self,
        name: &str,
        version: &str,
        api: ComponentApi,
    ) -> Result<(), &'static str> {
        if self.services.contains_key(name) {
            return Err("a different extension with this name is already registered");
        }
        if let Some(current) = self.components.get(name) {
            if current.version != version
                || current.api.create as usize != api.create as usize
                || current.api.update as usize != api.update as usize
                || current.api.render as usize != api.render as usize
                || current.api.event as usize != api.event as usize
                || current.api.destroy as usize != api.destroy as usize
            {
                return Err("a different extension with this name is already registered");
            }
            return Ok(());
        }
        if self.services.len() + self.components.len() >= abi::MAX_EXTENSIONS {
            return Err("native extension registry is full");
        }
        self.components.insert(
            name.to_owned(),
            RegisteredComponent {
                version: version.to_owned(),
                api,
            },
        );
        Ok(())
    }

    fn register(&mut self, name: &str, version: &str, api: ServiceApi) -> Result<(), &'static str> {
        if self.components.contains_key(name) {
            return Err("a different extension with this name is already registered");
        }
        if let Some(current) = self.services.get(name) {
            if current.version != version
                || current.api.invoke as usize != api.invoke as usize
                || current.api.shutdown as usize != api.shutdown as usize
            {
                return Err("a different extension with this name is already registered");
            }
            return Ok(());
        }
        if self.services.len() + self.components.len() >= abi::MAX_EXTENSIONS {
            return Err("native extension registry is full");
        }
        self.services.insert(
            name.to_owned(),
            RegisteredService {
                version: version.to_owned(),
                api,
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SHUTDOWNS: AtomicUsize = AtomicUsize::new(0);
    unsafe extern "C" fn invoke(_: u32, _: abi::Bytes, _: abi::Bytes, sink: abi::ServiceSink) {
        unsafe { (sink.release)(sink.context) };
    }
    unsafe extern "C" fn shutdown() {
        SHUTDOWNS.fetch_add(1, Ordering::SeqCst);
    }
    unsafe extern "C" fn other_shutdown() {
        SHUTDOWNS.fetch_add(10, Ordering::SeqCst);
    }
    static API: ServiceApi = ServiceApi { invoke, shutdown };

    #[test]
    fn accepts_independent_services_without_extension_specific_core_code() {
        let descriptor = Extension {
            abi_version: abi::ABI_VERSION,
            descriptor_size: size_of::<Extension>() as u32,
            kind: abi::SERVICE_EXTENSION,
            api_size: size_of::<ServiceApi>() as u32,
            name: abi::Bytes::new(b"independent-test-extension"),
            version: abi::Bytes::new(b"7.2.1"),
            api: ptr::addr_of!(API).cast(),
        };
        assert!(
            unsafe {
                register_extension_versioned(&descriptor, b"independent-test-extension", b"7.2.0")
            }
            .is_err()
        );
        for _ in 0..2 {
            unsafe {
                register_extension_versioned(&descriptor, b"independent-test-extension", b"7.2.1")
            }
            .unwrap();
        }
        assert_eq!(
            service("independent-test-extension").unwrap().invoke as usize,
            invoke as *const () as usize
        );
        assert!(service("unknown-test-extension").is_none());
        shutdown_services();
        assert_eq!(SHUTDOWNS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn bounds_registry_and_rejects_conflicting_versions_and_tables() {
        let mut registry = Registry::default();
        registry.register("one", "1.0.0", API).unwrap();
        registry.register("one", "1.0.0", API).unwrap();
        assert!(registry.register("one", "2.0.0", API).is_err());
        assert!(
            registry
                .register(
                    "one",
                    "1.0.0",
                    ServiceApi {
                        invoke,
                        shutdown: other_shutdown
                    }
                )
                .is_err()
        );
        for i in 1..abi::MAX_EXTENSIONS {
            registry
                .register(&format!("extension-{i}"), "1.0.0", API)
                .unwrap();
        }
        assert!(registry.register("overflow", "1.0.0", API).is_err());
    }

    #[test]
    fn rejects_invalid_service_names_kinds_and_null_callbacks() {
        let nulls = [0_usize; 2];
        let mut descriptor = Extension {
            abi_version: abi::ABI_VERSION,
            descriptor_size: size_of::<Extension>() as u32,
            kind: abi::SERVICE_EXTENSION,
            api_size: size_of::<ServiceApi>() as u32,
            name: abi::Bytes::new(b"extension/escape"),
            version: abi::Bytes::new(b"1.0.0"),
            api: ptr::addr_of!(API).cast(),
        };
        assert!(
            unsafe { register_extension_versioned(&descriptor, b"extension/escape", b"1.0.0") }
                .is_err()
        );
        descriptor.name = abi::Bytes::new(b"extension");
        descriptor.api = nulls.as_ptr().cast();
        assert_eq!(
            unsafe { register_extension_versioned(&descriptor, b"extension", b"1.0.0") },
            Err("service extension function table mismatch")
        );
        descriptor.api = ptr::addr_of!(API).cast();
        descriptor.kind = 999;
        assert!(
            unsafe { register_extension_versioned(&descriptor, b"extension", b"1.0.0") }.is_err()
        );
    }

    #[test]
    fn rejects_header_before_reading_an_unknown_descriptor_layout() {
        let header = [abi::ABI_VERSION + 1, 8];
        assert!(unsafe { register_extension(header.as_ptr().cast(), b"terminal") }.is_err());
        let short = [abi::ABI_VERSION, 8];
        assert!(unsafe { register_extension(short.as_ptr().cast(), b"terminal") }.is_err());
        assert!(unsafe { register_extension(ptr::null(), b"terminal") }.is_err());
    }
}
