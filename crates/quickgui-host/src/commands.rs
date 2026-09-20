//! JSON encoding of the system command surface.
//!
//! The application thread declares one request as a JSON object whose `method` names the command;
//! every parser here reuses the same bounded conversions the core-owned services expect, so the
//! JSON grammar adds no second source of truth. Results serialize back through the same structs.

use serde::Deserialize;

use super::*;
use crate::system::{
    NativeAboutPanelOptions, NativeClipboardItem, NativeNotificationOptions, NativeRelaunchOptions,
    NativeTrayIconOptions, NativeUserTask, SystemCommand, SystemCommandResult, TrayAction,
    about_panel_options, clipboard_item, file_icon_size, native_clipboard_item,
    parse_app_mutation_action, parse_app_service_action, parse_global_shortcut_action,
    parse_shell_action, parse_window_action, parse_window_image_action, system_notification,
    tray_options, user_tasks,
};

#[derive(Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case")]
enum SystemRequest {
    ConfigureApp {
        options: NativeAppOptions,
    },
    Exit,
    ExitWithCode {
        code: i32,
    },
    GetApplicationsFolderSupport,
    GetWindowRestoreState {
        window: u32,
    },
    AppService {
        request: u32,
        action: String,
        value: Option<String>,
    },
    AppMutation {
        action: String,
        value: Option<String>,
    },
    WindowPopupMenu {
        request: u32,
        window: u32,
        menu: String,
        x: Option<f64>,
        y: Option<f64>,
    },
    Relaunch {
        options: NativeRelaunchOptions,
    },
    GetAppInfo,
    GetAppPaths,
    GetSystemInfo,
    GetWindowRegistry,
    GetCursorScreenPosition,
    GetDesktopIntegrationSupport,
    GetSystemPreferences,
    GetDisplays,
    GetKeyboardLayout,
    GetWindowState {
        window: u32,
    },
    GetWindowFrameMetrics {
        window: u32,
    },
    ReadClipboard,
    WriteClipboard {
        item: NativeClipboardItem,
    },
    ShowNotification {
        options: NativeNotificationOptions,
    },
    DismissNotification {
        tag: String,
    },
    NotificationPermission {
        request: u32,
        prompt: bool,
    },
    SetDockBadge {
        value: Option<String>,
    },
    SetDockIcon {
        icon: Option<NativeImageSource>,
    },
    SetDockMenu {
        menu: Option<String>,
    },
    AddRecentDocument {
        path: String,
    },
    ClearRecentDocuments,
    ShowAboutPanel {
        options: NativeAboutPanelOptions,
    },
    FileIcon {
        request: u32,
        path: String,
        size: String,
    },
    SetUserTasks {
        request: u32,
        tasks: Vec<NativeUserTask>,
    },
    SetApplicationMenu {
        menu: String,
    },
    SetQuitInterception {
        intercepting: bool,
    },
    RequestQuit,
    ReadFindClipboard,
    WriteFindClipboard {
        item: NativeClipboardItem,
    },
    RequestSingleInstanceLock {
        identifier: String,
    },
    ReleaseSingleInstanceLock,
    GlobalShortcut {
        request: u32,
        action: String,
        registration: Option<u32>,
        accelerator: Option<String>,
    },
    SetTrayIcon {
        request: u32,
        options: NativeTrayIconOptions,
    },
    RemoveTrayIcon {
        request: u32,
        id: u32,
    },
    ShowTrayMenu {
        request: u32,
        id: u32,
    },
    WindowAction {
        window: u32,
        action: String,
        value: Option<String>,
    },
    WindowImageAction {
        window: u32,
        action: String,
        image: Option<NativeImageSource>,
        description: Option<String>,
    },
    ShellAction {
        request: u32,
        action: String,
        value: String,
    },
}

/// Longest JSON request accepted from the application thread.
const MAX_REQUEST_JSON_BYTES: usize = 4 * 1024 * 1024;

pub(crate) fn parse_system_request(json: &str) -> std::result::Result<SystemCommand, String> {
    if json.len() > MAX_REQUEST_JSON_BYTES {
        return Err("a native system request exceeds 4 MiB".to_owned());
    }
    let request: SystemRequest = serde_json::from_str(json)
        .map_err(|error| format!("invalid native system request: {error}"))?;
    Ok(match request {
        SystemRequest::ConfigureApp { options } => SystemCommand::ConfigureApp(options),
        SystemRequest::Exit => SystemCommand::Exit,
        SystemRequest::ExitWithCode { code } => SystemCommand::ExitWithCode(code),
        SystemRequest::GetApplicationsFolderSupport => SystemCommand::GetApplicationsFolderSupport,
        SystemRequest::GetWindowRestoreState { window } => {
            SystemCommand::GetWindowRestoreState(window)
        }
        SystemRequest::AppService {
            request,
            action,
            value,
        } => SystemCommand::AppService {
            request,
            action: parse_app_service_action(&action, value)?,
        },
        SystemRequest::AppMutation { action, value } => {
            SystemCommand::AppMutation(parse_app_mutation_action(&action, value)?)
        }
        SystemRequest::WindowPopupMenu {
            request,
            window,
            menu,
            x,
            y,
        } => SystemCommand::WindowPopupMenu {
            request,
            window,
            menu,
            position: popup_menu_position(x, y)?,
        },
        SystemRequest::Relaunch { options } => SystemCommand::Relaunch(options),
        SystemRequest::GetAppInfo => SystemCommand::GetAppInfo,
        SystemRequest::GetAppPaths => SystemCommand::GetAppPaths,
        SystemRequest::GetSystemInfo => SystemCommand::GetSystemInfo,
        SystemRequest::GetWindowRegistry => SystemCommand::GetWindowRegistry,
        SystemRequest::GetCursorScreenPosition => SystemCommand::GetCursorScreenPosition,
        SystemRequest::GetDesktopIntegrationSupport => SystemCommand::GetDesktopIntegrationSupport,
        SystemRequest::GetSystemPreferences => SystemCommand::GetSystemPreferences,
        SystemRequest::GetDisplays => SystemCommand::GetDisplays,
        SystemRequest::GetKeyboardLayout => SystemCommand::GetKeyboardLayout,
        SystemRequest::GetWindowState { window } => SystemCommand::GetWindowState(window),
        SystemRequest::GetWindowFrameMetrics { window } => {
            SystemCommand::GetWindowFrameMetrics(window)
        }
        SystemRequest::ReadClipboard => SystemCommand::ReadClipboard,
        SystemRequest::WriteClipboard { item } => {
            SystemCommand::WriteClipboard(clipboard_item(item)?)
        }
        SystemRequest::ShowNotification { options } => {
            SystemCommand::ShowNotification(system_notification(options)?)
        }
        SystemRequest::DismissNotification { tag } => SystemCommand::DismissNotification(tag),
        SystemRequest::NotificationPermission { request, prompt } => {
            SystemCommand::NotificationPermission { request, prompt }
        }
        SystemRequest::SetDockBadge { value } => SystemCommand::SetDockBadge(value),
        SystemRequest::SetDockIcon { icon } => {
            SystemCommand::SetDockIcon(icon.map(native_image).transpose()?)
        }
        SystemRequest::SetDockMenu { menu } => SystemCommand::SetDockMenu(menu),
        SystemRequest::AddRecentDocument { path } => {
            SystemCommand::AddRecentDocument(PathBuf::from(path))
        }
        SystemRequest::ClearRecentDocuments => SystemCommand::ClearRecentDocuments,
        SystemRequest::ShowAboutPanel { options } => {
            SystemCommand::ShowAboutPanel(about_panel_options(options)?)
        }
        SystemRequest::FileIcon {
            request,
            path,
            size,
        } => SystemCommand::FileIcon {
            request,
            path: PathBuf::from(path),
            size: file_icon_size(&size)?,
        },
        SystemRequest::SetUserTasks { request, tasks } => SystemCommand::SetUserTasks {
            request,
            tasks: user_tasks(tasks)?,
        },
        SystemRequest::SetApplicationMenu { menu } => SystemCommand::SetApplicationMenu(menu),
        SystemRequest::SetQuitInterception { intercepting } => {
            SystemCommand::SetQuitInterception(intercepting)
        }
        SystemRequest::RequestQuit => SystemCommand::RequestQuit,
        SystemRequest::ReadFindClipboard => SystemCommand::ReadFindClipboard,
        SystemRequest::WriteFindClipboard { item } => {
            SystemCommand::WriteFindClipboard(clipboard_item(item)?)
        }
        SystemRequest::RequestSingleInstanceLock { identifier } => {
            SystemCommand::RequestSingleInstanceLock(identifier)
        }
        SystemRequest::ReleaseSingleInstanceLock => SystemCommand::ReleaseSingleInstanceLock,
        SystemRequest::GlobalShortcut {
            request,
            action,
            registration,
            accelerator,
        } => SystemCommand::GlobalShortcut {
            request,
            action: parse_global_shortcut_action(&action, registration, accelerator)?,
        },
        SystemRequest::SetTrayIcon { request, options } => SystemCommand::Tray {
            request,
            action: TrayAction::Set(tray_options(options)?),
        },
        SystemRequest::RemoveTrayIcon { request, id } => SystemCommand::Tray {
            request,
            action: TrayAction::Remove(id),
        },
        SystemRequest::ShowTrayMenu { request, id } => SystemCommand::Tray {
            request,
            action: TrayAction::ShowMenu(id),
        },
        SystemRequest::WindowAction {
            window,
            action,
            value,
        } => SystemCommand::WindowAction {
            window,
            action: parse_window_action(&action, value)?,
        },
        SystemRequest::WindowImageAction {
            window,
            action,
            image,
            description,
        } => SystemCommand::WindowAction {
            window,
            action: parse_window_image_action(&action, image, description)?,
        },
        SystemRequest::ShellAction {
            request,
            action,
            value,
        } => SystemCommand::ShellAction {
            request,
            action: parse_shell_action(&action, value)?,
        },
    })
}

fn popup_menu_position(
    x: Option<f64>,
    y: Option<f64>,
) -> std::result::Result<Option<Point>, String> {
    match (x, y) {
        (None, None) => Ok(None),
        (Some(x), Some(y)) if x.is_finite() && y.is_finite() => {
            Ok(Some(Point::new(x as f32, y as f32)))
        }
        _ => Err("a native popup menu position requires finite x and y coordinates".to_owned()),
    }
}

/// Serialize one command result. Every variant is a plain data struct the application decodes.
pub(crate) fn system_result_json(result: SystemCommandResult) -> serde_json::Value {
    fn json<T: serde::Serialize>(value: &T) -> serde_json::Value {
        serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
    }
    match result {
        SystemCommandResult::Unit => serde_json::Value::Null,
        SystemCommandResult::Boolean(value) => serde_json::Value::Bool(value),
        SystemCommandResult::AppInfo(value) => json(&value),
        SystemCommandResult::AppPaths(value) => json(&value),
        SystemCommandResult::SystemInfo(value) => json(&value),
        SystemCommandResult::WindowRegistry(value) => json(&value),
        SystemCommandResult::Point(value) => json(&value),
        SystemCommandResult::DesktopIntegrationSupport(value) => json(&value),
        SystemCommandResult::SystemPreferences(value) => json(&value),
        SystemCommandResult::Displays(value) => json(&value),
        SystemCommandResult::KeyboardLayout(value) => json(&value),
        SystemCommandResult::WindowState(value) => json(&value),
        SystemCommandResult::FrameMetrics(value) => json(&value),
        SystemCommandResult::WindowRestoreState(value) => json(&value),
        SystemCommandResult::ApplicationsFolderSupport(value) => json(&value),
        SystemCommandResult::Clipboard(value) => json(&value.map(native_clipboard_item)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_requests_parse_into_bounded_system_commands() {
        let command = parse_system_request(
            r#"{"method":"window-action","window":3,"action":"set-title","value":"Hello"}"#,
        )
        .unwrap();
        assert!(matches!(
            command,
            SystemCommand::WindowAction {
                window: 3,
                action: crate::system::WindowAction::SetTitle(title),
            } if title == "Hello"
        ));

        let command = parse_system_request(
            r#"{"method":"window-popup-menu","request":9,"window":2,"menu":"[{\"label\":\"Context\",\"items\":[]}]","x":10,"y":20}"#,
        )
        .unwrap();
        assert!(matches!(
            command,
            SystemCommand::WindowPopupMenu {
                request: 9,
                window: 2,
                position: Some(position),
                ..
            } if position == Point::new(10.0, 20.0)
        ));

        let command =
            parse_system_request(r#"{"method":"get-window-frame-metrics","window":7}"#).unwrap();
        assert!(matches!(command, SystemCommand::GetWindowFrameMetrics(7)));

        assert!(parse_system_request(r#"{"method":"no-such-command"}"#).is_err());
        let error = parse_system_request(
            r#"{"method":"window-popup-menu","request":1,"window":2,"menu":"[]","x":1}"#,
        )
        .err()
        .expect("a half-specified position is rejected");
        assert!(error.contains("finite x and y"));
    }

    #[test]
    fn results_serialize_as_plain_json() {
        assert_eq!(
            system_result_json(SystemCommandResult::Unit),
            serde_json::Value::Null
        );
        assert_eq!(
            system_result_json(SystemCommandResult::Boolean(true)),
            serde_json::Value::Bool(true)
        );
        let point = system_result_json(SystemCommandResult::Point(crate::system::NativePoint {
            x: 1.5,
            y: 2.0,
        }));
        assert_eq!(point["x"], 1.5);
        assert_eq!(point["y"], 2.0);

        let metrics = system_result_json(SystemCommandResult::FrameMetrics(
            crate::system::NativeFrameMetrics {
                frame_number: 12,
                cpu_milliseconds: 1.25,
                smoothed_cpu_milliseconds: 1.5,
                frame_milliseconds: 8.0,
                smoothed_frame_milliseconds: 10.0,
            },
        ));
        assert_eq!(
            metrics,
            serde_json::json!({
                "frameNumber": 12,
                "cpuMilliseconds": 1.25,
                "smoothedCpuMilliseconds": 1.5,
                "frameMilliseconds": 8.0,
                "smoothedFrameMilliseconds": 10.0,
            })
        );
    }
}
