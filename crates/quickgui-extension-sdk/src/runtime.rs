use crate::{abi, schema::*};
use serde_json::Value;
use std::{
    ffi::c_void,
    marker::PhantomData,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

struct WakeOwner(abi::Wake);
// The host contract permits these two callbacks on any worker thread.
unsafe impl Send for WakeOwner {}
unsafe impl Sync for WakeOwner {}
impl Drop for WakeOwner {
    fn drop(&mut self) {
        unsafe { (self.0.release)(self.0.context) };
    }
}

#[derive(Clone)]
pub struct WakeHandle(Arc<WakeOwner>);
impl WakeHandle {
    /// # Safety
    /// Transfers the callback context exactly once; it must implement the ABI contract.
    pub unsafe fn from_raw(wake: abi::Wake) -> Self {
        Self(Arc::new(WakeOwner(wake)))
    }
    pub fn invalidate(&self) {
        unsafe { (self.0.0.wake)(self.0.0.context) };
    }
}

pub trait Component {
    /// Return false for equivalent properties. Source parsing belongs here, never in scroll.
    fn update(&mut self, props: Value) -> Result<bool, String>;
    fn render(&mut self, request: RenderRequest) -> Result<Frame, String>;
    fn event(&mut self, _event: InputEvent) -> Result<EventResult, String> {
        Ok(EventResult::default())
    }
}

pub trait ComponentFactory {
    fn create(name: &str, props: Value, wake: WakeHandle) -> Result<Box<dyn Component>, String>;
}

struct Instance {
    component: Box<dyn Component>,
    _wake: WakeHandle,
}

pub struct ComponentRuntime<F: ComponentFactory>(PhantomData<F>);
impl<F: ComponentFactory> ComponentRuntime<F> {
    pub const API: abi::ComponentApi = abi::ComponentApi {
        create: create::<F>,
        update,
        render,
        event,
        destroy,
    };
}

unsafe fn bytes<'a>(value: abi::Bytes) -> Result<&'a [u8], String> {
    if value.len > MAX_COMPONENT_BYTES || (value.len > 0 && value.data.is_null()) {
        return Err("invalid or oversized component payload".into());
    }
    if value.len == 0 {
        return Ok(&[]);
    }
    Ok(unsafe { std::slice::from_raw_parts(value.data, value.len) })
}

unsafe fn json<T: serde::de::DeserializeOwned>(value: abi::Bytes) -> Result<T, String> {
    serde_json::from_slice(unsafe { bytes(value)? }).map_err(|error| error.to_string())
}

unsafe fn reply_json<T: serde::Serialize>(
    value: &T,
    context: *mut c_void,
    reply: abi::Reply,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_COMPONENT_BYTES {
        return Err("component reply is too large".into());
    }
    unsafe { reply(context, abi::Bytes::new(&bytes)) };
    Ok(())
}

unsafe extern "C" fn create<F: ComponentFactory>(
    name: abi::Bytes,
    props: abi::Bytes,
    wake: abi::Wake,
    context: *mut c_void,
    reply: abi::Reply,
) -> *mut c_void {
    let wake = unsafe { WakeHandle::from_raw(wake) };
    let result = catch_unwind(AssertUnwindSafe(|| {
        let name =
            std::str::from_utf8(unsafe { bytes(name)? }).map_err(|error| error.to_string())?;
        let component = F::create(name, unsafe { json(props)? }, wake.clone())?;
        Ok::<_, String>(
            Box::into_raw(Box::new(Instance {
                component,
                _wake: wake,
            }))
            .cast(),
        )
    }));
    match result {
        Ok(Ok(instance)) => instance,
        error => {
            let message = match error {
                Ok(Err(message)) => message,
                _ => "component creation panicked".into(),
            };
            unsafe { reply(context, abi::Bytes::new(message.as_bytes())) };
            std::ptr::null_mut()
        }
    }
}

unsafe extern "C" fn update(handle: *mut c_void, props: abi::Bytes) -> i32 {
    if handle.is_null() {
        return -1;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        let instance = unsafe { &mut *handle.cast::<Instance>() };
        instance.component.update(unsafe { json(props)? })
    })) {
        Ok(Ok(changed)) => i32::from(changed),
        _ => -1,
    }
}

unsafe extern "C" fn render(
    handle: *mut c_void,
    request: abi::Bytes,
    context: *mut c_void,
    reply: abi::Reply,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        let instance = unsafe { &mut *handle.cast::<Instance>() };
        let request: RenderRequest = unsafe { json(request)? };
        if request.end.saturating_sub(request.start) > MAX_COMPONENT_ROWS {
            return Err("too many component rows requested".into());
        }
        let frame = instance.component.render(request)?;
        frame.validate().map_err(str::to_owned)?;
        unsafe { reply_json(&frame, context, reply) }
    })) {
        Ok(Ok(())) => 0,
        _ => -1,
    }
}

unsafe extern "C" fn event(
    handle: *mut c_void,
    event: abi::Bytes,
    context: *mut c_void,
    reply: abi::Reply,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        let instance = unsafe { &mut *handle.cast::<Instance>() };
        let result = instance.component.event(unsafe { json(event)? })?;
        unsafe { reply_json(&result, context, reply) }
    })) {
        Ok(Ok(())) => 0,
        _ => -1,
    }
}

unsafe extern "C" fn destroy(handle: *mut c_void) {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            drop(unsafe { Box::from_raw(handle.cast::<Instance>()) });
        }));
    }
}
