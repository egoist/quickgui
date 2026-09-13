use std::sync::Arc;

use windows::{
    Win32::{
        Foundation::HWND as WindowsHwnd,
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
        UI::{
            Shell::{
                ITaskbarList3, TBPF_ERROR, TBPF_INDETERMINATE, TBPF_NOPROGRESS, TBPF_NORMAL,
                TBPF_PAUSED, TaskbarList,
            },
            WindowsAndMessaging::{CreateIcon, DestroyIcon, HICON},
        },
    },
    core::{HSTRING, PCWSTR},
};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, LWA_ALPHA, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOOWNERZORDER,
    SWP_NOSIZE, SWP_NOZORDER, SetLayeredWindowAttributes, SetWindowPos, WS_EX_LAYERED,
    WS_EX_NOACTIVATE,
};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use crate::{Displays, Image, PlatformError, Point, TaskbarProgressState};

pub(super) fn current_cursor_screen_position(displays: &Displays) -> Result<Point, PlatformError> {
    let mut native = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
    // SAFETY: `native` is valid writable storage for the duration of the value-only query.
    if unsafe { GetCursorPos(&mut native) } == 0 {
        return Err(PlatformError::Unavailable);
    }
    let physical = Point::new(native.x as f32, native.y as f32);
    let display = displays
        .all()
        .iter()
        .find(|display| {
            let bounds = display.bounds();
            let scale = display.scale_factor();
            crate::Rect::new(
                bounds.x * scale,
                bounds.y * scale,
                bounds.width * scale,
                bounds.height * scale,
            )
            .contains(physical)
        })
        .or_else(|| displays.primary());
    let scale = display.map_or(1.0, crate::Display::scale_factor);
    Ok(Point::new(physical.x / scale, physical.y / scale))
}

fn hwnd(window: &Window) -> Result<HWND, String> {
    let handle = window.window_handle().map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("the window does not expose a Win32 handle".to_owned());
    };
    Ok(handle.hwnd.get() as HWND)
}

fn taskbar(window: &Window) -> Result<(ITaskbarList3, WindowsHwnd), String> {
    let hwnd = WindowsHwnd(hwnd(window)?);
    // SAFETY: Winit initializes OLE/COM on the window thread before a concrete Win32 window is
    // exposed. The created in-process COM object is released by its generated RAII wrapper.
    let taskbar: ITaskbarList3 = unsafe {
        CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| error.to_string())?
    };
    unsafe { taskbar.HrInit() }.map_err(|error| error.to_string())?;
    Ok((taskbar, hwnd))
}

pub(super) fn set_taskbar_progress(
    window: &Arc<Window>,
    state: TaskbarProgressState,
    progress: f32,
) -> Result<(), String> {
    if !progress.is_finite() || !(0.0..=1.0).contains(&progress) {
        return Err("taskbar progress must be finite and between zero and one".to_owned());
    }
    let (taskbar, hwnd) = taskbar(window)?;
    let native_state = match state {
        TaskbarProgressState::None => TBPF_NOPROGRESS,
        TaskbarProgressState::Normal => TBPF_NORMAL,
        TaskbarProgressState::Indeterminate => TBPF_INDETERMINATE,
        TaskbarProgressState::Paused => TBPF_PAUSED,
        TaskbarProgressState::Error => TBPF_ERROR,
    };
    // SAFETY: `hwnd` belongs to the retained window and the value parameters retain no pointers.
    unsafe {
        taskbar
            .SetProgressState(hwnd, native_state)
            .map_err(|error| error.to_string())?;
        if !matches!(
            state,
            TaskbarProgressState::None | TaskbarProgressState::Indeterminate
        ) {
            taskbar
                .SetProgressValue(
                    hwnd,
                    (f64::from(progress) * 10_000.0).round() as u64,
                    10_000,
                )
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub(super) fn set_taskbar_overlay_icon(
    window: &Arc<Window>,
    icon: Option<&Image>,
    description: Option<&str>,
) -> Result<(), String> {
    let (taskbar, hwnd) = taskbar(window)?;
    let native_icon = icon.map(create_native_icon).transpose()?;
    let description = description.map(HSTRING::from);
    // SAFETY: The taskbar copies the icon synchronously. The RAII guard destroys our HICON only
    // after the COM call returns, and the UTF-16 description lives for the complete call.
    let result = unsafe {
        taskbar.SetOverlayIcon(
            hwnd,
            native_icon
                .as_ref()
                .map_or_else(HICON::default, |icon| icon.0),
            description
                .as_ref()
                .map_or_else(PCWSTR::null, |value| PCWSTR(value.as_ptr())),
        )
    }
    .map_err(|error| error.to_string());
    drop(native_icon);
    result
}

pub(super) struct OwnedIcon(HICON);

impl OwnedIcon {
    pub(super) const fn handle(&self) -> HICON {
        self.0
    }
}

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        // SAFETY: `self.0` was created by `CreateIcon` and is destroyed exactly once here.
        let _ = unsafe { DestroyIcon(self.0) };
    }
}

pub(super) fn create_native_icon(image: &Image) -> Result<OwnedIcon, String> {
    let width = i32::try_from(image.width()).map_err(|_| "icon width is too large".to_owned())?;
    let height =
        i32::try_from(image.height()).map_err(|_| "icon height is too large".to_owned())?;
    let row_bytes = usize::try_from(image.width())
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or_else(|| "icon row is too large".to_owned())?;
    let mut bgra = vec![0_u8; image.byte_len()];
    for (source_row, target_row) in image
        .rgba()
        .chunks_exact(row_bytes)
        .zip(bgra.chunks_exact_mut(row_bytes).rev())
    {
        for (source, target) in source_row
            .chunks_exact(4)
            .zip(target_row.chunks_exact_mut(4))
        {
            let alpha = u16::from(source[3]);
            target[0] = ((u16::from(source[2]) * alpha + 127) / 255) as u8;
            target[1] = ((u16::from(source[1]) * alpha + 127) / 255) as u8;
            target[2] = ((u16::from(source[0]) * alpha + 127) / 255) as u8;
            target[3] = source[3];
        }
    }
    let mask_row_bytes = usize::try_from((image.width() + 31) / 32)
        .ok()
        .and_then(|words| words.checked_mul(4))
        .ok_or_else(|| "icon mask row is too large".to_owned())?;
    let mask = vec![0_u8; mask_row_bytes.saturating_mul(image.height() as usize)];
    // SAFETY: Both bit buffers are fully initialized and sized for the supplied dimensions. The
    // returned HICON owns a native copy and is wrapped immediately for deterministic destruction.
    unsafe { CreateIcon(None, width, height, 1, 32, mask.as_ptr(), bgra.as_ptr()) }
        .map(OwnedIcon)
        .map_err(|error| error.to_string())
}

unsafe fn get_window_long(window: HWND, index: i32) -> isize {
    #[cfg(target_pointer_width = "64")]
    {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(window, index) }
    }
    #[cfg(target_pointer_width = "32")]
    {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongW(window, index) as isize
        }
    }
}

unsafe fn set_window_long(window: HWND, index: i32, value: isize) {
    #[cfg(target_pointer_width = "64")]
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(window, index, value);
    }
    #[cfg(target_pointer_width = "32")]
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongW(window, index, value as i32);
    }
}

pub(super) fn set_window_focusable(window: &Arc<Window>, focusable: bool) -> Result<(), String> {
    let window = hwnd(window)?;
    // SAFETY: The raw handle belongs to the live Winit window and these style operations execute
    // on its event-loop thread.
    unsafe {
        let mut style = get_window_long(window, GWL_EXSTYLE) as u32;
        if focusable {
            style &= !WS_EX_NOACTIVATE;
        } else {
            style |= WS_EX_NOACTIVATE;
        }
        set_window_long(window, GWL_EXSTYLE, style as isize);
        SetWindowPos(
            window,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER,
        );
    }
    Ok(())
}

/// Block or restore every native input event for one window.
///
/// This is the direct Win32 equivalent of QuickGUI's `set_window_enabled`. It is a small,
/// obviously-correct addition that has not been verified on a live Windows desktop.
pub(super) fn set_window_input_enabled(window: &Arc<Window>, enabled: bool) -> Result<(), String> {
    let window = hwnd(window)?;
    // SAFETY: The raw handle belongs to the live Winit window and `EnableWindow` is called on its
    // event-loop thread.
    unsafe { EnableWindow(window, i32::from(enabled)) };
    Ok(())
}

pub(super) fn set_window_opacity(window: &Arc<Window>, opacity: f32) -> Result<(), String> {
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err("window opacity must be finite and between zero and one".to_owned());
    }
    let window = hwnd(window)?;
    let alpha = (opacity * 255.0).round() as u8;
    // SAFETY: The raw handle belongs to the live Winit window. Layered alpha is a documented
    // top-level-window operation and does not retain either pointer.
    unsafe {
        let style = get_window_long(window, GWL_EXSTYLE) as u32 | WS_EX_LAYERED;
        set_window_long(window, GWL_EXSTYLE, style as isize);
        if SetLayeredWindowAttributes(window, 0, alpha, LWA_ALPHA) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}
