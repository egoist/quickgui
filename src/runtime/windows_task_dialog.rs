//! Portable alerts on Windows, presented by the Common Controls v6 task dialog.
//!
//! Only Common Controls v6 exports `TaskDialogIndirect`, and Windows binds a module to v6 only
//! through an activation context that asks for it. A static import would require that manifest in
//! every executable or library containing the core, which Go, Bun, and Cargo binaries do not
//! carry, so the runtime creates its own v6 context and resolves the function on first use.

use std::{
    future::Future,
    os::windows::ffi::OsStrExt,
    pin::Pin,
    ptr,
    sync::{Arc, Mutex, OnceLock, PoisonError},
    task::{Context, Poll, Waker},
};

use windows_sys::{
    Win32::{
        Foundation::{HANDLE, HWND, INVALID_HANDLE_VALUE},
        System::{
            ApplicationInstallationAndServicing::{
                ACTCTXW, ActivateActCtx, CreateActCtxW, DeactivateActCtx,
            },
            LibraryLoader::{GetProcAddress, LoadLibraryW},
        },
        UI::{
            Controls::{
                TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TD_ERROR_ICON,
                TD_INFORMATION_ICON, TD_WARNING_ICON, TDF_ALLOW_DIALOG_CANCELLATION,
                TDF_SIZE_TO_CONTENT,
            },
            WindowsAndMessaging::IDCANCEL,
        },
    },
    core::{BOOL, HRESULT},
    s, w,
};

use crate::{PlatformError, PromptButton, PromptLevel};

const COMMON_CONTROLS_V6_MANIFEST: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">"#,
    r#"<dependency><dependentAssembly><assemblyIdentity type="win32" "#,
    r#"name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" "#,
    r#"publicKeyToken="6595b64144ccf1df" language="*"/></dependentAssembly></dependency>"#,
    r#"</assembly>"#,
);

/// Custom button identifiers stay clear of the stock `ID*` results such as `IDCANCEL`.
const FIRST_BUTTON_ID: i32 = 1000;

type TaskDialogIndirect =
    unsafe extern "system" fn(*const TASKDIALOGCONFIG, *mut i32, *mut i32, *mut BOOL) -> HRESULT;

struct CommonControls {
    context: HANDLE,
    task_dialog: TaskDialogIndirect,
}

// SAFETY: The activation context is never released and may be activated on any thread; the
// function pointer refers to a module that stays loaded for the process lifetime.
unsafe impl Send for CommonControls {}
unsafe impl Sync for CommonControls {}

fn common_controls() -> Result<&'static CommonControls, PlatformError> {
    static COMMON_CONTROLS: OnceLock<Result<CommonControls, PlatformError>> = OnceLock::new();
    COMMON_CONTROLS
        .get_or_init(load_common_controls)
        .as_ref()
        .map_err(Clone::clone)
}

fn load_common_controls() -> Result<CommonControls, PlatformError> {
    // CreateActCtxW reads manifests only from files, and the context keeps no reference to it.
    let path = std::env::temp_dir().join(format!(
        "quickgui-common-controls-{}.manifest",
        std::process::id()
    ));
    std::fs::write(&path, COMMON_CONTROLS_V6_MANIFEST).map_err(platform_error)?;
    let source = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let request = ACTCTXW {
        cbSize: size_of::<ACTCTXW>() as u32,
        lpSource: source.as_ptr(),
        ..Default::default()
    };
    // SAFETY: `request` and the NUL-terminated path it names outlive the call.
    let context = unsafe { CreateActCtxW(&request) };
    let created = std::io::Error::last_os_error();
    let _ = std::fs::remove_file(&path);
    if context == INVALID_HANDLE_VALUE {
        return Err(platform_error(created));
    }
    // SAFETY: Loading by name under the v6 context binds the side-by-side Common Controls.
    let module = activated(context, || unsafe { LoadLibraryW(w!("comctl32.dll")) })?;
    if module.is_null() {
        return Err(platform_error(std::io::Error::last_os_error()));
    }
    // SAFETY: `module` is a loaded Common Controls image and the name is NUL-terminated.
    let task_dialog =
        unsafe { GetProcAddress(module, s!("TaskDialogIndirect")) }.ok_or_else(|| {
            PlatformError::Platform("Common Controls v6 does not export TaskDialogIndirect".into())
        })?;
    Ok(CommonControls {
        context,
        // SAFETY: Every Common Controls v6 release exports TaskDialogIndirect with this signature.
        task_dialog: unsafe {
            std::mem::transmute::<unsafe extern "system" fn() -> isize, TaskDialogIndirect>(
                task_dialog,
            )
        },
    })
}

/// Run `body` with `context` active on the calling thread.
fn activated<T>(context: HANDLE, body: impl FnOnce() -> T) -> Result<T, PlatformError> {
    let mut cookie = 0;
    // SAFETY: The context handle stays valid for the process lifetime.
    if unsafe { ActivateActCtx(context, &mut cookie) } == 0 {
        return Err(platform_error(std::io::Error::last_os_error()));
    }
    let result = body();
    // SAFETY: `cookie` belongs to the activation above, on this thread.
    unsafe { DeactivateActCtx(0, cookie) };
    Ok(result)
}

fn platform_error(error: std::io::Error) -> PlatformError {
    PlatformError::Platform(format!("Common Controls v6 is unavailable: {error}").into())
}

/// Present a task dialog on its own thread, as RFD does, so the event loop keeps running.
///
/// The response is the chosen button index, or `None` when the user dismissed the dialog.
pub(super) fn show(
    parent: Option<HWND>,
    level: PromptLevel,
    title: &str,
    description: &str,
    buttons: &[PromptButton],
) -> TaskDialogResponse {
    let request = TaskDialogRequest {
        parent: parent.unwrap_or(ptr::null_mut()),
        level,
        title: wide(title),
        description: wide(description),
        buttons: buttons.iter().map(|button| wide(button.label())).collect(),
    };
    let reply = Arc::new(Reply::default());
    let sender = reply.clone();
    let spawned = std::thread::Builder::new()
        .name("quickgui-task-dialog".to_owned())
        .spawn(move || {
            let result = std::panic::catch_unwind(|| request.present()).unwrap_or_else(|_| {
                Err(PlatformError::Platform("the task dialog panicked".into()))
            });
            sender.complete(result);
        });
    if let Err(error) = spawned {
        reply.complete(Err(PlatformError::Platform(error.to_string().into())));
    }
    TaskDialogResponse(reply)
}

struct TaskDialogRequest {
    parent: HWND,
    level: PromptLevel,
    title: Vec<u16>,
    description: Vec<u16>,
    buttons: Vec<Vec<u16>>,
}

// SAFETY: A window handle is a process-wide identifier that user32 accepts from any thread.
unsafe impl Send for TaskDialogRequest {}

impl TaskDialogRequest {
    fn present(&self) -> Result<Option<usize>, PlatformError> {
        let controls = common_controls()?;
        let buttons = self
            .buttons
            .iter()
            .zip(FIRST_BUTTON_ID..)
            .map(|(label, id)| TASKDIALOG_BUTTON {
                nButtonID: id,
                pszButtonText: label.as_ptr(),
            })
            .collect::<Vec<_>>();
        let config = TASKDIALOGCONFIG {
            cbSize: size_of::<TASKDIALOGCONFIG>() as u32,
            hwndParent: self.parent,
            dwFlags: TDF_ALLOW_DIALOG_CANCELLATION | TDF_SIZE_TO_CONTENT,
            pszWindowTitle: self.title.as_ptr(),
            Anonymous1: TASKDIALOGCONFIG_0 {
                pszMainIcon: match self.level {
                    PromptLevel::Info => TD_INFORMATION_ICON,
                    PromptLevel::Warning => TD_WARNING_ICON,
                    PromptLevel::Critical => TD_ERROR_ICON,
                },
            },
            pszContent: self.description.as_ptr(),
            cButtons: buttons.len() as u32,
            pButtons: buttons.as_ptr(),
            ..Default::default()
        };
        let mut chosen = 0;
        // SAFETY: The configuration, the strings and buttons it points to, and `chosen` outlive
        // the modal call; the radio-button and verification results are optional.
        let status = activated(controls.context, || unsafe {
            (controls.task_dialog)(&config, &mut chosen, ptr::null_mut(), ptr::null_mut())
        })?;
        if status < 0 {
            return Err(PlatformError::Platform(
                format!("the task dialog failed with HRESULT {status:#010x}").into(),
            ));
        }
        if chosen == IDCANCEL {
            return Ok(None);
        }
        chosen
            .checked_sub(FIRST_BUTTON_ID)
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < self.buttons.len())
            .map(Some)
            .ok_or_else(|| {
                PlatformError::Platform("the native alert returned an unknown button".into())
            })
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

#[derive(Default)]
struct Reply {
    state: Mutex<ReplyState>,
}

#[derive(Default)]
struct ReplyState {
    result: Option<Result<Option<usize>, PlatformError>>,
    waker: Option<Waker>,
}

impl Reply {
    fn complete(&self, result: Result<Option<usize>, PlatformError>) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.result = Some(result);
        let waker = state.waker.take();
        drop(state);
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// Resolves when the user closes the task dialog.
pub(super) struct TaskDialogResponse(Arc<Reply>);

impl Future for TaskDialogResponse {
    type Output = Result<Option<usize>, PlatformError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.state.lock().unwrap_or_else(PoisonError::into_inner);
        match state.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
