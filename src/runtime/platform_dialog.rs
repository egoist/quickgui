use super::*;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::{
    SystemNotificationActionKind, SystemNotificationSound,
    platform::MAX_SYSTEM_NOTIFICATION_REPLY_BYTES,
};

#[cfg(target_os = "macos")]
pub(super) struct ActivePlatformDialog {
    pub(super) id: PlatformDialogId,
    pub(super) native: MacPlatformDialog,
    pub(super) focus: Option<MacPlatformDialogFocus>,
}

#[cfg(target_os = "windows")]
fn show_portable_system_notification(
    notification: SystemNotification,
    proxy: EventLoopProxy<RuntimeEvent>,
    app_id: Option<&str>,
) -> Result<(), PlatformError> {
    use std::fmt::Write as _;
    use windows::{
        Data::Xml::Dom::XmlDocument,
        Foundation::{DateTime, IPropertyValue, TypedEventHandler},
        UI::Notifications::{
            ScheduledToastNotification, ToastActivatedEventArgs, ToastNotification,
            ToastNotificationManager,
        },
        core::{HSTRING, IInspectable, Interface},
    };

    let app_id = windows_notification_app_id(app_id)?;
    windows_shell::set_current_app_id(app_id)?;

    let mut actions = String::new();
    let mut reply_inputs = Vec::new();
    if !notification.actions.is_empty() {
        actions.push_str("<actions>");
        for (index, action) in notification.actions.iter().enumerate() {
            match &action.kind {
                SystemNotificationActionKind::Button => {
                    let _ = write!(
                        actions,
                        "<action content=\"{}\" arguments=\"{}\"/>",
                        escape_notification_xml(&action.label),
                        escape_notification_xml(&action.id),
                    );
                }
                SystemNotificationActionKind::TextInput { placeholder } => {
                    let input = format!("quickgui-reply-{index}");
                    let _ = write!(
                        actions,
                        "<input id=\"{input}\" type=\"text\" placeHolderContent=\"{}\"/><action content=\"{}\" arguments=\"{}\" hint-inputId=\"{input}\"/>",
                        escape_notification_xml(placeholder.as_deref().unwrap_or("")),
                        escape_notification_xml(&action.label),
                        escape_notification_xml(&action.id),
                    );
                    reply_inputs.push((action.id.clone(), input));
                }
            }
        }
        actions.push_str("</actions>");
    }

    let mut images = String::new();
    if let Some(icon) = &notification.icon {
        let source = windows_notification_file_uri(icon)?;
        let _ = write!(
            images,
            "<image placement=\"appLogoOverride\" src=\"{}\"/>",
            escape_notification_xml(&source),
        );
    }
    for attachment in &notification.attachments {
        let source = windows_notification_file_uri(&attachment.path)?;
        let _ = write!(
            images,
            "<image src=\"{}\" alt=\"{}\"/>",
            escape_notification_xml(&source),
            escape_notification_xml(&attachment.id),
        );
    }

    let mut texts = String::new();
    let _ = write!(
        texts,
        "<text>{}</text>",
        escape_notification_xml(&notification.title),
    );
    if let Some(subtitle) = &notification.subtitle {
        let _ = write!(texts, "<text>{}</text>", escape_notification_xml(subtitle),);
    }
    let _ = write!(
        texts,
        "<text>{}</text>",
        escape_notification_xml(&notification.body),
    );

    let audio = match &notification.sound {
        SystemNotificationSound::Default => String::new(),
        SystemNotificationSound::Silent => "<audio silent=\"true\"/>".to_owned(),
        SystemNotificationSound::Named(name) => {
            format!("<audio src=\"{}\"/>", escape_notification_xml(name))
        }
    };
    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\">{texts}{images}</binding></visual>{actions}{audio}</toast>",
    );
    let document = XmlDocument::new().map_err(windows_platform_error)?;
    document
        .LoadXml(&HSTRING::from(xml))
        .map_err(windows_platform_error)?;
    let native_tag = HSTRING::from(windows_notification_tag(&notification.tag));
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))
        .map_err(windows_platform_error)?;

    if let Some(delivery_at) = notification
        .delivery_at
        .filter(|delivery_at| *delivery_at > web_time::SystemTime::now())
    {
        let scheduled = ScheduledToastNotification::CreateScheduledToastNotification(
            &document,
            DateTime {
                UniversalTime: windows_notification_time(delivery_at)?,
            },
        )
        .map_err(windows_platform_error)?;
        scheduled
            .SetTag(&native_tag)
            .map_err(windows_platform_error)?;
        remove_scheduled_windows_notifications(&notifier, &native_tag)?;
        notifier
            .AddToSchedule(&scheduled)
            .map_err(windows_platform_error)?;
        return Ok(());
    }

    let toast =
        ToastNotification::CreateToastNotification(&document).map_err(windows_platform_error)?;
    toast.SetTag(&native_tag).map_err(windows_platform_error)?;

    let tag = notification.tag;
    let activated = TypedEventHandler::<ToastNotification, IInspectable>::new(move |_, value| {
        let arguments = value
            .as_ref()
            .and_then(|value| value.cast::<ToastActivatedEventArgs>().ok());
        let action_id = arguments
            .as_ref()
            .and_then(|arguments| arguments.Arguments().ok())
            .map(|arguments| arguments.to_string())
            .filter(|arguments| !arguments.is_empty())
            .map(Arc::<str>::from);
        let reply = action_id
            .as_ref()
            .and_then(|action_id| {
                reply_inputs
                    .iter()
                    .find(|(candidate, _)| candidate == action_id)
            })
            .and_then(|(_, input)| {
                arguments
                    .as_ref()?
                    .UserInput()
                    .ok()?
                    .Lookup(&HSTRING::from(input))
                    .ok()?
                    .cast::<IPropertyValue>()
                    .ok()?
                    .GetString()
                    .ok()
            })
            .map(|reply| reply.to_string())
            .filter(|reply| {
                reply.len() <= MAX_SYSTEM_NOTIFICATION_REPLY_BYTES && !reply.contains('\0')
            })
            .map(Arc::<str>::from);
        let _ = proxy.send_event(RuntimeEvent::SystemNotificationResponse(
            SystemNotificationResponse {
                tag: tag.clone(),
                action_id,
                reply,
            },
        ));
        Ok(())
    });
    toast
        .Activated(&activated)
        .map_err(windows_platform_error)?;
    notifier.Show(&toast).map_err(windows_platform_error)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn dismiss_windows_system_notification(
    tag: &str,
    app_id: Option<&str>,
) -> Result<(), PlatformError> {
    use windows::{UI::Notifications::ToastNotificationManager, core::HSTRING};

    let app_id = windows_notification_app_id(app_id)?;
    windows_shell::set_current_app_id(app_id)?;
    let native_tag = HSTRING::from(windows_notification_tag(tag));
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))
        .map_err(windows_platform_error)?;
    remove_scheduled_windows_notifications(&notifier, &native_tag)?;

    ToastNotificationManager::History()
        .and_then(|history| {
            history.RemoveGroupedTagWithId(&native_tag, &HSTRING::new(), &HSTRING::from(app_id))
        })
        .map_err(windows_platform_error)
}

#[cfg(target_os = "windows")]
fn windows_notification_app_id(app_id: Option<&str>) -> Result<&str, PlatformError> {
    app_id.ok_or_else(|| {
        PlatformError::Platform(
            "Windows system notifications require AppInfo with an application identifier".into(),
        )
    })
}

#[cfg(target_os = "windows")]
fn remove_scheduled_windows_notifications(
    notifier: &windows::UI::Notifications::ToastNotifier,
    tag: &windows::core::HSTRING,
) -> Result<(), PlatformError> {
    let scheduled = notifier
        .GetScheduledToastNotifications()
        .map_err(windows_platform_error)?;
    for index in 0..scheduled.Size().map_err(windows_platform_error)? {
        let notification = scheduled.GetAt(index).map_err(windows_platform_error)?;
        if notification.Tag().ok().as_ref() == Some(tag) {
            notifier
                .RemoveFromSchedule(&notification)
                .map_err(windows_platform_error)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_notification_time(time: web_time::SystemTime) -> Result<i64, PlatformError> {
    const WINDOWS_TO_UNIX_EPOCH_SECONDS: u64 = 11_644_473_600;
    const TICKS_PER_SECOND: u64 = 10_000_000;

    let unix = time
        .duration_since(web_time::UNIX_EPOCH)
        .map_err(|_| PlatformError::InvalidNotificationOptions)?;
    let seconds = unix
        .as_secs()
        .checked_add(WINDOWS_TO_UNIX_EPOCH_SECONDS)
        .and_then(|seconds| seconds.checked_mul(TICKS_PER_SECOND))
        .ok_or(PlatformError::InvalidNotificationOptions)?;
    let ticks = seconds
        .checked_add(u64::from(unix.subsec_nanos()) / 100)
        .and_then(|ticks| i64::try_from(ticks).ok())
        .ok_or(PlatformError::InvalidNotificationOptions)?;
    Ok(ticks)
}

#[cfg(target_os = "windows")]
fn windows_notification_file_uri(path: &std::path::Path) -> Result<String, PlatformError> {
    let path = std::path::absolute(path)
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    let path = dunce::simplified(&path)
        .to_str()
        .ok_or(PlatformError::InvalidNotificationOptions)?
        .replace('\\', "/");
    let mut uri = if path.starts_with("//") {
        format!("file:{path}")
    } else {
        format!("file:///{path}")
    };
    let prefix = uri.find(':').map_or(0, |index| index + 1);
    let unescaped = uri.split_off(prefix);
    for byte in unescaped.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/' | b':') {
            uri.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(uri, "%{byte:02X}");
        }
    }
    Ok(uri)
}

#[cfg(target_os = "windows")]
fn portable_notification_permission_status(
    app_id: Option<&str>,
) -> Result<NotificationPermissionStatus, PlatformError> {
    use windows::{
        UI::Notifications::{NotificationSetting, ToastNotificationManager},
        core::HSTRING,
    };

    let app_id = windows_notification_app_id(app_id)?;
    windows_shell::set_current_app_id(app_id)?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))
        .map_err(windows_platform_error)?;
    let setting = notifier.Setting().map_err(windows_platform_error)?;
    Ok(if setting == NotificationSetting::Enabled {
        NotificationPermissionStatus::Granted
    } else {
        NotificationPermissionStatus::Denied
    })
}

#[cfg(target_os = "windows")]
fn windows_platform_error(error: windows::core::Error) -> PlatformError {
    PlatformError::Platform(error.to_string().into())
}

#[cfg(target_os = "windows")]
fn windows_notification_tag(tag: &str) -> String {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in tag.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    format!("{hash:016x}")
}

#[cfg(target_os = "windows")]
fn escape_notification_xml(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
pub(super) struct ActivePlatformDialog {
    pub(super) id: PlatformDialogId,
    pub(super) _task: Task<()>,
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
struct PlatformDialogCompletion {
    proxy: EventLoopProxy<RuntimeEvent>,
    owner: Option<WindowHandle>,
    id: PlatformDialogId,
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
impl Drop for PlatformDialogCompletion {
    fn drop(&mut self) {
        let _ = self
            .proxy
            .send_event(RuntimeEvent::PlatformDialogClosed(self.owner, self.id));
    }
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
async fn pick_rfd_paths(
    dialog: rfd::AsyncFileDialog,
    files: bool,
    directories: bool,
    multiple: bool,
) -> Result<Option<Vec<PathBuf>>, PlatformError> {
    let selected = match (files, directories, multiple) {
        (true, false, false) => dialog.pick_file().await.map(|path| vec![path]),
        (true, false, true) => dialog.pick_files().await,
        (false, true, false) => dialog.pick_folder().await.map(|path| vec![path]),
        (false, true, true) => dialog.pick_folders().await,
        // RFD only exposes the combined file-or-folder picker on macOS. QuickGUI's AppKit
        // backend handles that case there; other desktop targets fail explicitly.
        _ => return Err(PlatformError::Unsupported),
    };
    selected
        .map(|selected| {
            bounded_selected_paths(
                selected
                    .into_iter()
                    .map(|path| path.path().to_path_buf())
                    .collect(),
            )
        })
        .transpose()
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn bounded_selected_paths(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, PlatformError> {
    if paths.len() > crate::MAX_SELECTED_PATHS {
        return Err(PlatformError::SelectionTooLarge);
    }
    let mut total_bytes = 0_usize;
    for path in &paths {
        let bytes = path.as_os_str().as_encoded_bytes();
        if bytes.is_empty() || bytes.len() > crate::MAX_PLATFORM_PATH_BYTES {
            return Err(PlatformError::SelectionTooLarge);
        }
        total_bytes = total_bytes.saturating_add(bytes.len());
        if total_bytes > crate::MAX_SELECTED_PATHS_TOTAL_BYTES {
            return Err(PlatformError::SelectionTooLarge);
        }
    }
    Ok(paths)
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn bounded_selected_path(path: PathBuf) -> Result<PathBuf, PlatformError> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if bytes.is_empty() || bytes.len() > crate::MAX_PLATFORM_PATH_BYTES {
        Err(PlatformError::SelectionTooLarge)
    } else {
        Ok(path)
    }
}

impl Runtime {
    pub(super) fn process_platform_requests(&mut self) {
        #[cfg(target_os = "macos")]
        // AppKit completes the response before the queued close event is dispatched. Keep that
        // completed dialog alive until `PlatformDialogClosed` removes it so its saved responder
        // cannot be lost between those two event-loop boundaries.
        self.active_platform_dialogs.retain(|owner, _| {
            owner
                .as_ref()
                .is_none_or(|handle| self.window_handles.contains_key(handle))
        });

        while let Some(request) = self.platform_requests.pop_front() {
            if request.response_cancelled() {
                continue;
            }
            let Some(request) = self.apply_tray_platform_request(request) else {
                continue;
            };
            #[cfg(target_os = "macos")]
            self.process_macos_platform_request(request);
            #[cfg(any(
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            self.process_rfd_platform_request(request);
            #[cfg(not(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            )))]
            request.complete_error(PlatformError::Unsupported);
        }
    }

    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn process_rfd_platform_request(&mut self, request: PlatformRequest) {
        match request {
            PlatformRequest::Prompt {
                window,
                level,
                message,
                detail,
                buttons,
                responder,
            } => self.start_rfd_prompt(window, level, message, detail, buttons, responder),
            PlatformRequest::OpenPaths {
                window,
                options,
                responder,
            } => self.start_rfd_open_dialog(window, options, responder),
            PlatformRequest::MessageBox {
                window,
                options,
                responder,
            } => self.start_rfd_message_box(window, *options, responder),
            PlatformRequest::PreviewFile { responder, .. }
            | PlatformRequest::CloseFilePreview { responder }
            | PlatformRequest::ShowColorPanel { responder, .. }
            | PlatformRequest::CloseColorPanel { responder }
            | PlatformRequest::ShowFontPanel { responder, .. }
            | PlatformRequest::ShareItems { responder, .. } => {
                finish_shell_request(
                    responder,
                    Err(PlatformError::Unsupported),
                    "unsupported native panel",
                );
            }
            PlatformRequest::AuthenticateWithBiometrics { responder, .. } => {
                responder.complete(Err(PlatformError::Unsupported));
            }
            PlatformRequest::SavePath {
                window,
                options,
                responder,
            } => self.start_rfd_save_dialog(window, options, responder),
            PlatformRequest::ShowSystemNotification(notification) => {
                if let Err(error) = show_portable_system_notification(
                    notification,
                    self.event_proxy.clone(),
                    self.app_info.as_ref().map(AppInfo::identifier),
                ) {
                    tracing::warn!(%error, "could not show system notification");
                }
            }
            PlatformRequest::DismissSystemNotification(tag) => {
                #[cfg(target_os = "windows")]
                if let Err(error) = dismiss_windows_system_notification(
                    &tag,
                    self.app_info.as_ref().map(AppInfo::identifier),
                ) {
                    tracing::warn!(%error, %tag, "could not dismiss system notification");
                }
                #[cfg(target_os = "linux")]
                if let Err(error) = dismiss_linux_system_notification(&tag) {
                    tracing::warn!(%error, %tag, "could not dismiss system notification");
                }
                #[cfg(not(any(target_os = "windows", target_os = "linux")))]
                tracing::warn!(%tag, "dismissing notifications is not supported by this backend");
            }
            PlatformRequest::NotificationPermissionStatus { responder }
            | PlatformRequest::RequestNotificationPermission { responder } => {
                responder.complete(portable_notification_permission_status(
                    self.app_info.as_ref().map(AppInfo::identifier),
                ));
            }
            PlatformRequest::OpenUrl { url, responder } => finish_shell_request(
                responder,
                opener::open(url.as_ref())
                    .map_err(|error| PlatformError::Platform(error.to_string().into())),
                "open URL",
            ),
            PlatformRequest::OpenPath { path, responder } => finish_shell_request(
                responder,
                opener::open(&path)
                    .map_err(|error| PlatformError::Platform(error.to_string().into())),
                "open path",
            ),
            PlatformRequest::RevealPath { path, responder } => finish_shell_request(
                responder,
                opener::reveal(&path)
                    .map_err(|error| PlatformError::Platform(error.to_string().into())),
                "reveal path",
            ),
            PlatformRequest::TrashPath { path, responder } => {
                finish_shell_request(responder, move_path_to_trash(&path), "move path to trash")
            }
            PlatformRequest::SetDockBadge(value) => {
                let _ = value;
                tracing::warn!("macOS Dock integration is not supported by this backend");
            }
            PlatformRequest::SetDockIcon(icon) => {
                let _ = icon;
                tracing::warn!("macOS Dock integration is not supported by this backend");
            }
            PlatformRequest::SetDockMenu(menu) => {
                let _ = menu;
                tracing::warn!("macOS Dock integration is not supported by this backend");
            }
            PlatformRequest::SetTrayIcon(_)
            | PlatformRequest::RemoveTrayIcon(_)
            | PlatformRequest::ShowTrayMenu(_) => {
                unreachable!("tray requests are applied before platform dispatch")
            }
            PlatformRequest::AddRecentDocument(path) => {
                #[cfg(target_os = "windows")]
                windows_shell::add_recent_document(&path);
                #[cfg(not(target_os = "windows"))]
                tracing::warn!(?path, "recent documents are not supported by this backend");
            }
            PlatformRequest::ClearRecentDocuments => {
                #[cfg(target_os = "windows")]
                windows_shell::clear_recent_documents();
                #[cfg(not(target_os = "windows"))]
                tracing::warn!("recent documents are not supported by this backend");
            }
            PlatformRequest::ShowAboutPanel(mut options) => {
                if let Some(info) = &self.app_info {
                    options
                        .application_name
                        .get_or_insert_with(|| info.name().into());
                    options
                        .application_version
                        .get_or_insert_with(|| info.version().into());
                }
                #[cfg(target_os = "windows")]
                if let Err(error) = windows_shell::show_about_panel(&options) {
                    tracing::warn!(%error, "could not show the native About panel");
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = options;
                    tracing::warn!("a native About panel is not supported by this backend");
                }
            }
            PlatformRequest::GetFileIcon {
                path,
                size,
                responder,
            } => {
                #[cfg(target_os = "windows")]
                responder.complete(windows_shell::file_icon(&path, size));
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = (path, size);
                    responder.complete(Err(PlatformError::Unsupported));
                }
            }
            PlatformRequest::SetUserTasks { tasks, responder } => {
                #[cfg(target_os = "windows")]
                responder.complete(windows_shell::set_user_tasks(
                    &tasks,
                    self.app_info.as_ref().map(AppInfo::identifier),
                ));
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = tasks;
                    responder.complete(Err(PlatformError::Unsupported));
                }
            }
            PlatformRequest::SetActivationPolicy { policy, responder } => {
                let _ = policy;
                responder.complete(Err(PlatformError::Unsupported));
            }
            PlatformRequest::ActivateApplication { force } => {
                let _ = force;
                tracing::warn!("application activation is not supported by this backend");
            }
            PlatformRequest::HideApplication | PlatformRequest::UnhideApplication => {
                tracing::warn!("application hiding is not supported by this backend");
            }
            PlatformRequest::RequestDockAttention {
                attention,
                responder,
            } => {
                let _ = attention;
                responder.complete(Err(PlatformError::Unsupported));
            }
            PlatformRequest::CancelDockAttention(request) => {
                let _ = request;
            }
            PlatformRequest::SetDockVisible { visible, responder } => {
                let _ = visible;
                responder.complete(Err(PlatformError::Unsupported));
            }
            PlatformRequest::SetSecureKeyboardEntry(enabled) => {
                let _ = enabled;
                tracing::warn!("secure keyboard entry is not supported by this backend");
            }
            PlatformRequest::Beep => {
                tracing::warn!("a system alert sound is not supported by this backend");
            }
            PlatformRequest::MoveToApplicationsFolder { responder } => {
                responder.complete(Err(PlatformError::Unsupported));
            }
        }
    }

    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn rfd_parent_window(
        &self,
        owner: Option<WindowHandle>,
    ) -> Result<Option<Arc<Window>>, PlatformError> {
        self.validate_platform_dialog_owner(owner)?;
        let Some(owner) = owner else {
            return Ok(None);
        };
        let window_id = self
            .window_handles
            .get(&owner)
            .copied()
            .ok_or(PlatformError::Unavailable)?;
        self.windows
            .get(&window_id)
            .map(|entry| Some(entry.state.window.clone()))
            .ok_or(PlatformError::Unavailable)
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn validate_platform_dialog_owner(
        &self,
        owner: Option<WindowHandle>,
    ) -> Result<(), PlatformError> {
        let busy = self.active_platform_dialogs.keys().any(|candidate| {
            match (owner, *candidate) {
                (Some(owner), Some(candidate)) => self.windows_share_parent_chain(owner, candidate),
                // An application-modal dialog is exclusive across the whole application.
                (None, _) | (_, None) => true,
            }
        });
        if busy {
            return Err(PlatformError::DialogBusy);
        }
        if self.active_platform_dialogs.len() == crate::MAX_ACTIVE_PLATFORM_DIALOGS {
            return Err(PlatformError::TooManyDialogs);
        }
        Ok(())
    }

    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn start_rfd_prompt(
        &mut self,
        owner: Option<WindowHandle>,
        level: PromptLevel,
        message: Arc<str>,
        detail: Option<Arc<str>>,
        buttons: Vec<PromptButton>,
        responder: crate::platform::PlatformResponder<usize>,
    ) {
        let native_window = match self.rfd_parent_window(owner) {
            Ok(window) => window,
            Err(error) => {
                responder.complete(Err(error));
                return;
            }
        };
        let native_buttons = match rfd_prompt_buttons(&buttons) {
            Ok(buttons) => buttons,
            Err(error) => {
                responder.complete(Err(error));
                return;
            }
        };

        let mut dialog = rfd::AsyncMessageDialog::new()
            .set_level(match level {
                PromptLevel::Info => rfd::MessageLevel::Info,
                PromptLevel::Warning => rfd::MessageLevel::Warning,
                PromptLevel::Critical => rfd::MessageLevel::Error,
            })
            .set_buttons(native_buttons);
        if let Some(native_window) = &native_window {
            dialog = dialog.set_parent(native_window.as_ref());
        }
        // RFD has a title and one description field rather than AppKit's message/detail pair.
        // Preserve the hierarchy when a detail exists; otherwise avoid repeating the message.
        dialog = if let Some(detail) = &detail {
            dialog
                .set_title(message.as_ref())
                .set_description(detail.as_ref())
        } else {
            dialog.set_description(message.as_ref())
        };

        let id = PlatformDialogId::next();
        let completion = PlatformDialogCompletion {
            proxy: self.event_proxy.clone(),
            owner,
            id,
        };
        let failed_responder = responder.clone();
        let future = async move {
            let _completion = completion;
            let result = dialog.show().await;
            responder.complete(rfd_prompt_index(result, &buttons));
        };
        let task = match owner {
            Some(owner) => self
                .foreground_tasks
                .spawn::<(), _, _, _>(owner, move |_| future),
            None => self.foreground_tasks.spawn_application(future),
        };
        match task {
            Ok(task) => {
                self.active_platform_dialogs
                    .insert(owner, ActivePlatformDialog { id, _task: task });
            }
            Err(error) => {
                failed_responder.complete(Err(PlatformError::Platform(error.to_string().into())))
            }
        }
    }

    /// Present a portable message box.
    ///
    /// The portable backend has no suppression checkbox and no custom icon, so those options are
    /// rejected rather than silently dropped. The response always reports `checkbox_checked:
    /// false` here.
    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn start_rfd_message_box(
        &mut self,
        owner: Option<WindowHandle>,
        options: crate::MessageBoxOptions,
        responder: crate::platform::PlatformResponder<crate::MessageBoxResponse>,
    ) {
        if options.checkbox.is_some() || options.icon.is_some() {
            responder.complete(Err(PlatformError::Unsupported));
            return;
        }
        let native_window = match self.rfd_parent_window(owner) {
            Ok(window) => window,
            Err(error) => {
                responder.complete(Err(error));
                return;
            }
        };
        let buttons = options.resolved_buttons();
        let native_buttons = match rfd_prompt_buttons(&buttons) {
            Ok(buttons) => buttons,
            Err(error) => {
                responder.complete(Err(error));
                return;
            }
        };

        let mut dialog = rfd::AsyncMessageDialog::new()
            .set_level(match options.level.unwrap_or(PromptLevel::Info) {
                PromptLevel::Info => rfd::MessageLevel::Info,
                PromptLevel::Warning => rfd::MessageLevel::Warning,
                PromptLevel::Critical => rfd::MessageLevel::Error,
            })
            .set_buttons(native_buttons);
        if let Some(native_window) = &native_window {
            dialog = dialog.set_parent(native_window.as_ref());
        }
        dialog = if let Some(detail) = &options.detail {
            dialog
                .set_title(options.message.as_ref())
                .set_description(detail.as_ref())
        } else {
            dialog.set_description(options.message.as_ref())
        };

        let id = PlatformDialogId::next();
        let completion = PlatformDialogCompletion {
            proxy: self.event_proxy.clone(),
            owner,
            id,
        };
        let failed_responder = responder.clone();
        let future = async move {
            let _completion = completion;
            let result = dialog.show().await;
            responder.complete(rfd_prompt_index(result, &buttons).map(|button| {
                crate::MessageBoxResponse {
                    button,
                    checkbox_checked: false,
                }
            }));
        };
        let task = match owner {
            Some(owner) => self
                .foreground_tasks
                .spawn::<(), _, _, _>(owner, move |_| future),
            None => self.foreground_tasks.spawn_application(future),
        };
        match task {
            Ok(task) => {
                self.active_platform_dialogs
                    .insert(owner, ActivePlatformDialog { id, _task: task });
            }
            Err(error) => {
                failed_responder.complete(Err(PlatformError::Platform(error.to_string().into())))
            }
        }
    }

    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn start_rfd_open_dialog(
        &mut self,
        owner: Option<WindowHandle>,
        options: PathPromptOptions,
        responder: crate::platform::PlatformResponder<Option<Vec<PathBuf>>>,
    ) {
        let native_window = match self.rfd_parent_window(owner) {
            Ok(window) => window,
            Err(error) => {
                responder.complete(Err(error));
                return;
            }
        };
        if options.files && options.directories {
            responder.complete(Err(PlatformError::Unsupported));
            return;
        }

        let mut dialog = rfd::AsyncFileDialog::new();
        if let Some(native_window) = &native_window {
            dialog = dialog.set_parent(native_window.as_ref());
        }
        if let Some(directory) = &options.directory {
            dialog = dialog.set_directory(directory);
        }
        if let Some(name) = &options.suggested_name {
            dialog = dialog.set_file_name(name.as_ref());
        }
        if let Some(title) = &options.title {
            dialog = dialog.set_title(title.as_ref());
        }
        for filter in &options.filters {
            let extensions = filter
                .extensions
                .iter()
                .map(|extension| extension.as_ref())
                .collect::<Vec<&str>>();
            dialog = dialog.add_filter(filter.name.as_ref(), &extensions);
        }
        let files = options.files;
        let directories = options.directories;
        let multiple = options.multiple;
        let id = PlatformDialogId::next();
        let completion = PlatformDialogCompletion {
            proxy: self.event_proxy.clone(),
            owner,
            id,
        };
        let failed_responder = responder.clone();
        let future = async move {
            let _completion = completion;
            responder.complete(pick_rfd_paths(dialog, files, directories, multiple).await);
        };
        let task = match owner {
            Some(owner) => self
                .foreground_tasks
                .spawn::<(), _, _, _>(owner, move |_| future),
            None => self.foreground_tasks.spawn_application(future),
        };
        match task {
            Ok(task) => {
                self.active_platform_dialogs
                    .insert(owner, ActivePlatformDialog { id, _task: task });
            }
            Err(error) => {
                failed_responder.complete(Err(PlatformError::Platform(error.to_string().into())))
            }
        }
    }

    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn start_rfd_save_dialog(
        &mut self,
        owner: Option<WindowHandle>,
        options: SavePathOptions,
        responder: crate::platform::PlatformResponder<Option<PathBuf>>,
    ) {
        let native_window = match self.rfd_parent_window(owner) {
            Ok(window) => window,
            Err(error) => {
                responder.complete(Err(error));
                return;
            }
        };

        let mut dialog = rfd::AsyncFileDialog::new().set_directory(&options.directory);
        if let Some(native_window) = &native_window {
            dialog = dialog.set_parent(native_window.as_ref());
        }
        if let Some(name) = &options.suggested_name {
            dialog = dialog.set_file_name(name.as_ref());
        }
        if let Some(title) = &options.title {
            dialog = dialog.set_title(title.as_ref());
        }
        for filter in &options.filters {
            let extensions = filter
                .extensions
                .iter()
                .map(|extension| extension.as_ref())
                .collect::<Vec<&str>>();
            dialog = dialog.add_filter(filter.name.as_ref(), &extensions);
        }
        let id = PlatformDialogId::next();
        let completion = PlatformDialogCompletion {
            proxy: self.event_proxy.clone(),
            owner,
            id,
        };
        let failed_responder = responder.clone();
        let future = async move {
            let _completion = completion;
            let result = dialog
                .save_file()
                .await
                .map(|path| bounded_selected_path(path.path().to_path_buf()))
                .transpose();
            responder.complete(result);
        };
        let task = match owner {
            Some(owner) => self
                .foreground_tasks
                .spawn::<(), _, _, _>(owner, move |_| future),
            None => self.foreground_tasks.spawn_application(future),
        };
        match task {
            Ok(task) => {
                self.active_platform_dialogs
                    .insert(owner, ActivePlatformDialog { id, _task: task });
            }
            Err(error) => {
                failed_responder.complete(Err(PlatformError::Platform(error.to_string().into())))
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn process_macos_platform_request(&mut self, request: PlatformRequest) {
        match request {
            PlatformRequest::ShowSystemNotification(notification) => {
                if let Some(host) = self.mac_application_host.as_mut() {
                    host.show_system_notification(notification);
                }
            }
            PlatformRequest::DismissSystemNotification(tag) => {
                if let Some(host) = self.mac_application_host.as_mut() {
                    host.dismiss_system_notification(&tag);
                }
            }
            PlatformRequest::NotificationPermissionStatus { responder } => {
                if let Some(host) = self.mac_application_host.as_mut() {
                    host.system_notification_permission_status(responder);
                } else {
                    responder.complete(Ok(NotificationPermissionStatus::Unsupported));
                }
            }
            PlatformRequest::RequestNotificationPermission { responder } => {
                if let Some(host) = self.mac_application_host.as_mut() {
                    host.request_system_notification_permission(responder);
                } else {
                    responder.complete(Ok(NotificationPermissionStatus::Unsupported));
                }
            }
            PlatformRequest::OpenUrl { url, responder } => finish_shell_request(
                responder,
                shell_open_url(&url)
                    .map_err(|error| PlatformError::Platform(error.to_string().into())),
                "open URL",
            ),
            PlatformRequest::OpenPath { path, responder } => finish_shell_request(
                responder,
                shell_open_path(&path)
                    .map_err(|error| PlatformError::Platform(error.to_string().into())),
                "open path",
            ),
            PlatformRequest::RevealPath { path, responder } => finish_shell_request(
                responder,
                shell_reveal_path(&path)
                    .map_err(|error| PlatformError::Platform(error.to_string().into())),
                "reveal path",
            ),
            PlatformRequest::TrashPath { path, responder } => finish_shell_request(
                responder,
                shell_trash_path(&path)
                    .map_err(|error| PlatformError::Platform(error.to_string().into())),
                "move path to trash",
            ),
            PlatformRequest::SetDockBadge(value) => {
                match crate::macos_shell::set_dock_badge(value.as_deref()) {
                    Ok(()) => self.dock_badge = value,
                    Err(error) => tracing::warn!(%error, "could not change the Dock badge"),
                }
            }
            PlatformRequest::SetDockIcon(icon) => {
                match crate::macos_shell::set_dock_icon(icon.as_ref()) {
                    Ok(()) => self.dock_icon = icon,
                    Err(error) => tracing::warn!(%error, "could not change the Dock icon"),
                }
            }
            PlatformRequest::SetTrayIcon(_)
            | PlatformRequest::RemoveTrayIcon(_)
            | PlatformRequest::ShowTrayMenu(_) => {
                unreachable!("tray requests are applied before platform dispatch")
            }
            PlatformRequest::SetDockMenu(menu) => {
                let actions = menu
                    .as_ref()
                    .map(|menu| collect_menu_actions(std::slice::from_ref(menu)))
                    .unwrap_or_default();
                if let Some(host) = self.mac_application_host.as_mut() {
                    match host.set_dock_menu(menu.clone(), self.event_proxy.clone()) {
                        Ok(()) => {
                            self.dock_menu = menu;
                            self.dock_menu_actions = actions;
                        }
                        Err(error) => {
                            tracing::warn!(%error, "could not replace the Dock menu");
                        }
                    }
                }
            }
            PlatformRequest::AddRecentDocument(path) => {
                if let Err(error) = crate::macos_shell::add_recent_document(&path) {
                    tracing::warn!(%error, "could not add a recent document");
                }
            }
            PlatformRequest::ClearRecentDocuments => {
                if let Err(error) = crate::macos_shell::clear_recent_documents() {
                    tracing::warn!(%error, "could not clear recent documents");
                }
            }
            PlatformRequest::ShowAboutPanel(mut options) => {
                if let Some(info) = &self.app_info {
                    options
                        .application_name
                        .get_or_insert_with(|| info.name().into());
                    options
                        .application_version
                        .get_or_insert_with(|| info.version().into());
                }
                if let Err(error) = crate::macos_shell::show_about_panel(&options) {
                    tracing::warn!(%error, "could not show the native About panel");
                }
            }
            PlatformRequest::GetFileIcon {
                path,
                size,
                responder,
            } => responder.complete(crate::macos_shell::file_icon(&path, size)),
            PlatformRequest::SetUserTasks { tasks, responder } => {
                let _ = tasks;
                responder.complete(Err(PlatformError::Unsupported));
            }
            PlatformRequest::SetActivationPolicy { policy, responder } => {
                responder.complete(crate::macos_shell::set_activation_policy(policy));
            }
            PlatformRequest::ActivateApplication { force } => {
                if let Err(error) = crate::macos_shell::activate_application(force) {
                    tracing::warn!(%error, "could not activate the application");
                }
            }
            PlatformRequest::HideApplication => {
                if let Err(error) = crate::macos_shell::hide_application() {
                    tracing::warn!(%error, "could not hide the application");
                }
            }
            PlatformRequest::UnhideApplication => {
                if let Err(error) = crate::macos_shell::unhide_application() {
                    tracing::warn!(%error, "could not unhide the application");
                }
            }
            PlatformRequest::RequestDockAttention {
                attention,
                responder,
            } => responder.complete(crate::macos_shell::request_dock_attention(attention)),
            PlatformRequest::CancelDockAttention(request) => {
                if let Err(error) = crate::macos_shell::cancel_dock_attention(request) {
                    tracing::warn!(%error, "could not cancel a Dock attention request");
                }
            }
            PlatformRequest::SetDockVisible { visible, responder } => {
                responder.complete(crate::macos_shell::set_dock_visible(visible));
            }
            PlatformRequest::SetSecureKeyboardEntry(enabled) => {
                if let Err(error) = crate::macos_shell::set_secure_keyboard_entry(enabled) {
                    tracing::warn!(%error, "could not change secure keyboard entry");
                }
            }
            PlatformRequest::Beep => {
                if let Err(error) = crate::macos_shell::beep() {
                    tracing::warn!(%error, "could not play the system alert sound");
                }
            }
            PlatformRequest::MoveToApplicationsFolder { responder } => {
                responder.complete(crate::macos_shell::move_to_applications_folder());
            }
            PlatformRequest::PreviewFile {
                path,
                display_name,
                responder,
            } => finish_shell_request(
                responder,
                crate::macos::preview_file(&path, display_name.as_deref()),
                "preview file",
            ),
            PlatformRequest::CloseFilePreview { responder } => {
                finish_shell_request(
                    responder,
                    crate::macos::close_file_preview(),
                    "close preview",
                );
            }
            PlatformRequest::ShowColorPanel {
                initial,
                mode,
                responder,
            } => finish_shell_request(
                responder,
                crate::macos::show_color_panel(&self.event_proxy, initial, mode),
                "show color panel",
            ),
            PlatformRequest::CloseColorPanel { responder } => finish_shell_request(
                responder,
                crate::macos::close_color_panel(),
                "close color panel",
            ),
            PlatformRequest::ShowFontPanel { font, responder } => finish_shell_request(
                responder,
                crate::macos::show_font_panel(&self.event_proxy, &font),
                "show font panel",
            ),
            PlatformRequest::ShareItems {
                window,
                items,
                anchor,
                responder,
            } => {
                let native_window = window
                    .and_then(|window| self.window_handles.get(&window).copied())
                    .and_then(|window_id| self.windows.get(&window_id))
                    .map(|entry| entry.state.window.clone());
                let result = if window.is_some() && native_window.is_none() {
                    Err(PlatformError::Unavailable)
                } else {
                    crate::macos::share_items(native_window.as_ref(), &items, anchor)
                };
                finish_shell_request(responder, result, "share items");
            }
            PlatformRequest::AuthenticateWithBiometrics { reason, responder } => {
                crate::macos::authenticate_with_biometrics(&self.event_proxy, &reason, responder);
            }
            request => {
                let owner = request.window();
                let native_window = match owner {
                    Some(owner) => {
                        let Some(window_id) = self.window_handles.get(&owner).copied() else {
                            request.complete_error(PlatformError::Unavailable);
                            return;
                        };
                        let Some(native_window) = self
                            .windows
                            .get(&window_id)
                            .map(|entry| entry.state.window.clone())
                        else {
                            request.complete_error(PlatformError::Unavailable);
                            return;
                        };
                        Some(native_window)
                    }
                    None => None,
                };
                if let Err(error) = self.validate_platform_dialog_owner(owner) {
                    request.complete_error(error);
                    return;
                }

                let id = PlatformDialogId::next();
                let open = Arc::new(AtomicBool::new(true));
                if !request.bind_cancellation(self.event_proxy.clone(), owner, id) {
                    return;
                }
                let focus = match MacPlatformDialogFocus::capture(native_window.as_ref()) {
                    Ok(focus) => focus,
                    Err(error) => {
                        request.complete_error(PlatformError::Platform(error.into()));
                        return;
                    }
                };
                let context = MacPlatformDialogContext::new(
                    owner,
                    id,
                    open.clone(),
                    self.event_proxy.clone(),
                );
                let native = match request {
                    PlatformRequest::Prompt {
                        level,
                        message,
                        detail,
                        buttons,
                        responder,
                        ..
                    } => {
                        let completion = responder.clone();
                        match present_native_prompt(
                            native_window.as_ref(),
                            context,
                            level,
                            &message,
                            detail.as_deref(),
                            &buttons,
                            completion,
                        ) {
                            Ok(native) => native,
                            Err(error) => {
                                responder.complete(Err(PlatformError::Platform(error.into())));
                                return;
                            }
                        }
                    }
                    PlatformRequest::OpenPaths {
                        options, responder, ..
                    } => {
                        let completion = responder.clone();
                        match present_native_open_panel(
                            native_window.as_ref(),
                            context,
                            &options,
                            completion,
                        ) {
                            Ok(native) => native,
                            Err(error) => {
                                responder.complete(Err(PlatformError::Platform(error.into())));
                                return;
                            }
                        }
                    }
                    PlatformRequest::SavePath {
                        options, responder, ..
                    } => {
                        let completion = responder.clone();
                        match present_native_save_panel(
                            native_window.as_ref(),
                            context,
                            &options,
                            completion,
                        ) {
                            Ok(native) => native,
                            Err(error) => {
                                responder.complete(Err(PlatformError::Platform(error.into())));
                                return;
                            }
                        }
                    }
                    PlatformRequest::MessageBox {
                        options, responder, ..
                    } => {
                        let completion = responder.clone();
                        match crate::macos::present_native_message_box(
                            native_window.as_ref(),
                            context,
                            &options,
                            completion,
                        ) {
                            Ok(native) => native,
                            Err(error) => {
                                responder.complete(Err(PlatformError::Platform(error.into())));
                                return;
                            }
                        }
                    }
                    PlatformRequest::PreviewFile { .. }
                    | PlatformRequest::CloseFilePreview { .. }
                    | PlatformRequest::ShowColorPanel { .. }
                    | PlatformRequest::CloseColorPanel { .. }
                    | PlatformRequest::ShowFontPanel { .. }
                    | PlatformRequest::ShareItems { .. }
                    | PlatformRequest::AuthenticateWithBiometrics { .. }
                    | PlatformRequest::ShowSystemNotification(_)
                    | PlatformRequest::DismissSystemNotification(_)
                    | PlatformRequest::NotificationPermissionStatus { .. }
                    | PlatformRequest::RequestNotificationPermission { .. }
                    | PlatformRequest::OpenUrl { .. }
                    | PlatformRequest::OpenPath { .. }
                    | PlatformRequest::RevealPath { .. }
                    | PlatformRequest::TrashPath { .. }
                    | PlatformRequest::SetDockBadge(_)
                    | PlatformRequest::SetDockIcon(_)
                    | PlatformRequest::SetDockMenu(_)
                    | PlatformRequest::SetTrayIcon(_)
                    | PlatformRequest::RemoveTrayIcon(_)
                    | PlatformRequest::ShowTrayMenu(_)
                    | PlatformRequest::AddRecentDocument(_)
                    | PlatformRequest::ClearRecentDocuments
                    | PlatformRequest::ShowAboutPanel(_)
                    | PlatformRequest::GetFileIcon { .. }
                    | PlatformRequest::SetUserTasks { .. }
                    | PlatformRequest::SetActivationPolicy { .. }
                    | PlatformRequest::ActivateApplication { .. }
                    | PlatformRequest::HideApplication
                    | PlatformRequest::UnhideApplication
                    | PlatformRequest::RequestDockAttention { .. }
                    | PlatformRequest::CancelDockAttention(_)
                    | PlatformRequest::SetDockVisible { .. }
                    | PlatformRequest::SetSecureKeyboardEntry(_)
                    | PlatformRequest::Beep
                    | PlatformRequest::MoveToApplicationsFolder { .. } => {
                        unreachable!("application-wide platform actions returned above")
                    }
                };
                if open.load(Ordering::Acquire) {
                    self.active_platform_dialogs
                        .insert(owner, ActivePlatformDialog { id, native, focus });
                } else if let Some(focus) = focus
                    && !focus.restore()
                {
                    tracing::warn!("AppKit rejected the platform dialog's saved first responder");
                }
            }
        }
    }
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn rfd_prompt_buttons(buttons: &[PromptButton]) -> Result<rfd::MessageButtons, PlatformError> {
    if buttons.len() > 3 {
        return Err(PlatformError::Platform(
            "the portable native alert backend supports at most three buttons".into(),
        ));
    }
    if buttons.iter().enumerate().any(|(index, button)| {
        buttons[..index]
            .iter()
            .any(|candidate| candidate.label() == button.label())
    }) {
        return Err(PlatformError::Platform(
            "the portable native alert backend requires unique button labels".into(),
        ));
    }
    Ok(match buttons {
        [first] => rfd::MessageButtons::OkCustom(first.label().to_owned()),
        [first, second] => {
            rfd::MessageButtons::OkCancelCustom(first.label().to_owned(), second.label().to_owned())
        }
        [first, second, third] => rfd::MessageButtons::YesNoCancelCustom(
            first.label().to_owned(),
            second.label().to_owned(),
            third.label().to_owned(),
        ),
        _ => return Err(PlatformError::InvalidButtons),
    })
}

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn rfd_prompt_index(
    result: rfd::MessageDialogResult,
    buttons: &[PromptButton],
) -> Result<usize, PlatformError> {
    let index = match result {
        rfd::MessageDialogResult::Custom(label) => {
            buttons.iter().position(|button| button.label() == label)
        }
        rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes => Some(0),
        rfd::MessageDialogResult::No => (buttons.len() > 1).then_some(1),
        rfd::MessageDialogResult::Cancel => buttons
            .iter()
            .position(PromptButton::is_cancel)
            .or_else(|| buttons.len().checked_sub(1)),
    };
    index.ok_or_else(|| {
        PlatformError::Platform("the native alert returned an unknown button".into())
    })
}

#[cfg(target_os = "windows")]
fn move_path_to_trash(path: &Path) -> Result<(), PlatformError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::{
        FO_DELETE, FOF_ALLOWUNDO, FOF_NO_UI, FOF_WANTNUKEWARNING, SHFILEOPSTRUCTW, SHFileOperationW,
    };

    let path = dunce::canonicalize(path)
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    let mut source = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if source.contains(&0) {
        return Err(PlatformError::InvalidPath);
    }
    // SHFileOperation takes a double-NUL-terminated list even for one path.
    source.extend_from_slice(&[0, 0]);
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: source.as_ptr(),
        fFlags: (FOF_NO_UI | FOF_ALLOWUNDO | FOF_WANTNUKEWARNING) as u16,
        ..Default::default()
    };
    let status = unsafe { SHFileOperationW(&mut operation) };
    if status != 0 {
        return Err(PlatformError::Platform(
            format!("Windows recycle-bin operation failed with status {status}").into(),
        ));
    }
    if operation.fAnyOperationsAborted != 0 {
        return Err(PlatformError::Platform(
            "Windows recycle-bin operation was aborted".into(),
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn move_path_to_trash(path: &Path) -> Result<(), PlatformError> {
    use std::{fs::OpenOptions, os::fd::AsFd};

    let path = std::fs::canonicalize(path)
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    let portal_result = (|| {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .or_else(|_| OpenOptions::new().read(true).open(&path))?;
        let connection = zbus::blocking::Connection::session()
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let proxy = zbus::blocking::Proxy::new(
            &connection,
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Trash",
        )
        .map_err(|error| std::io::Error::other(error.to_string()))?;
        let status: u32 = proxy
            .call("TrashFile", &(zbus::zvariant::Fd::from(file.as_fd())))
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        if status == 1 {
            Ok(())
        } else {
            Err(std::io::Error::other(
                "the desktop trash portal rejected the path",
            ))
        }
    })();
    if portal_result.is_ok() || !path.exists() {
        return Ok(());
    }

    let gio_result = std::process::Command::new("gio")
        .args(["trash", "--"])
        .arg(&path)
        .status();
    if gio_result.is_ok_and(|status| status.success()) {
        return Ok(());
    }
    Err(PlatformError::Platform(
        format!(
            "could not move the path to trash through the desktop portal ({}) or gio",
            portal_result.expect_err("checked above")
        )
        .into(),
    ))
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn move_path_to_trash(path: &Path) -> Result<(), PlatformError> {
    let status = std::process::Command::new("gio")
        .args(["trash", "--"])
        .arg(path)
        .status()
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    if status.success() {
        Ok(())
    } else {
        Err(PlatformError::Platform(
            format!("gio trash exited with status {status}").into(),
        ))
    }
}

#[cfg(target_os = "linux")]
const MAX_LINUX_NOTIFICATION_CALLBACKS: usize = 256;

#[cfg(target_os = "linux")]
#[derive(Clone)]
struct LinuxNotificationRegistration {
    tag: Arc<str>,
    proxy: EventLoopProxy<RuntimeEvent>,
}

#[cfg(target_os = "linux")]
#[derive(Default)]
struct LinuxNotificationRegistrations {
    entries: HashMap<String, LinuxNotificationRegistration>,
    order: VecDeque<String>,
}

#[cfg(target_os = "linux")]
struct LinuxNotificationHub {
    connection: zbus::blocking::Connection,
    registrations: Arc<std::sync::Mutex<LinuxNotificationRegistrations>>,
}

#[cfg(target_os = "linux")]
impl LinuxNotificationHub {
    fn new() -> Result<Self, PlatformError> {
        let connection = zbus::blocking::Connection::session()
            .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
        let registrations = Arc::new(std::sync::Mutex::new(
            LinuxNotificationRegistrations::default(),
        ));
        let signal_connection = connection.clone();
        let signal_registrations = registrations.clone();
        let (ready_sender, ready_receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("quickgui-notifications".to_owned())
            .spawn(move || {
                let proxy = match linux_notification_proxy(&signal_connection) {
                    Ok(proxy) => proxy,
                    Err(error) => {
                        let _ = ready_sender.send(Err(error.to_string()));
                        return;
                    }
                };
                let signals = match proxy.receive_signal("ActionInvoked") {
                    Ok(signals) => signals,
                    Err(error) => {
                        let _ = ready_sender.send(Err(error.to_string()));
                        return;
                    }
                };
                let _ = ready_sender.send(Ok(()));
                for message in signals {
                    let Ok((id, action, parameters)) =
                        message
                            .body()
                            .deserialize::<(String, String, Vec<zbus::zvariant::OwnedValue>)>()
                    else {
                        continue;
                    };
                    let registration =
                        signal_registrations
                            .lock()
                            .ok()
                            .and_then(|mut registrations| {
                                registrations.order.retain(|queued| queued != &id);
                                registrations.entries.remove(&id)
                            });
                    let Some(registration) = registration else {
                        continue;
                    };
                    let action_id = (action != "default").then(|| Arc::<str>::from(action));
                    let reply = parameters
                        .into_iter()
                        .next()
                        .and_then(|value| String::try_from(value).ok())
                        .filter(|reply| {
                            reply.len() <= MAX_SYSTEM_NOTIFICATION_REPLY_BYTES
                                && !reply.contains('\0')
                        })
                        .map(Arc::<str>::from);
                    let _ =
                        registration
                            .proxy
                            .send_event(RuntimeEvent::SystemNotificationResponse(
                                SystemNotificationResponse {
                                    tag: registration.tag,
                                    action_id,
                                    reply,
                                },
                            ));
                }
            })
            .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
        ready_receiver
            .recv()
            .map_err(|error| PlatformError::Platform(error.to_string().into()))?
            .map_err(|error| PlatformError::Platform(error.into()))?;
        Ok(Self {
            connection,
            registrations,
        })
    }

    fn register(&self, id: String, registration: LinuxNotificationRegistration) -> Option<String> {
        let mut registrations = self.registrations.lock().ok()?;
        registrations.order.retain(|queued| queued != &id);
        let evicted = if registrations.entries.len() == MAX_LINUX_NOTIFICATION_CALLBACKS
            && !registrations.entries.contains_key(&id)
        {
            registrations.order.pop_front().inspect(|oldest| {
                registrations.entries.remove(oldest);
            })
        } else {
            None
        };
        registrations.order.push_back(id.clone());
        registrations.entries.insert(id, registration);
        evicted
    }

    fn unregister(&self, id: &str) {
        if let Ok(mut registrations) = self.registrations.lock() {
            registrations.order.retain(|queued| queued != id);
            registrations.entries.remove(id);
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_notification_hub() -> Result<&'static LinuxNotificationHub, PlatformError> {
    static HUB: std::sync::OnceLock<Result<LinuxNotificationHub, Arc<str>>> =
        std::sync::OnceLock::new();
    HUB.get_or_init(|| LinuxNotificationHub::new().map_err(|error| Arc::from(error.to_string())))
        .as_ref()
        .map_err(|error| PlatformError::Platform(error.clone()))
}

#[cfg(target_os = "linux")]
fn linux_notification_proxy(
    connection: &zbus::blocking::Connection,
) -> zbus::Result<zbus::blocking::Proxy<'_>> {
    zbus::blocking::Proxy::new(
        connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Notification",
    )
}

#[cfg(target_os = "linux")]
fn show_portable_system_notification(
    notification: SystemNotification,
    proxy: EventLoopProxy<RuntimeEvent>,
    _app_id: Option<&str>,
) -> Result<(), PlatformError> {
    use ashpd::desktop::{
        Icon,
        notification::{Button, ButtonPurpose, Notification},
    };

    if notification
        .delivery_at
        .is_some_and(|delivery_at| delivery_at > web_time::SystemTime::now())
        || !notification.attachments.is_empty()
        || matches!(&notification.sound, SystemNotificationSound::Named(_))
    {
        return Err(PlatformError::Unsupported);
    }

    let hub = linux_notification_hub()?;
    let id = notification.tag.to_string();
    let body = notification.subtitle.as_ref().map_or_else(
        || notification.body.to_string(),
        |subtitle| {
            if notification.body.is_empty() {
                subtitle.to_string()
            } else {
                format!("{subtitle}\n{}", notification.body)
            }
        },
    );
    let mut native = Notification::new(&notification.title)
        .body(body.as_str())
        .default_action("default");
    if let Some(icon) = &notification.icon {
        native = native.icon(Icon::Bytes(read_bounded_notification_icon(icon)?));
    }
    for action in &notification.actions {
        let mut button = Button::new(&action.label, &action.id);
        if matches!(&action.kind, SystemNotificationActionKind::TextInput { .. }) {
            button = button.purpose(ButtonPurpose::ImReplyWithText);
        }
        native = native.button(button);
    }
    let evicted = hub.register(
        id.clone(),
        LinuxNotificationRegistration {
            tag: notification.tag,
            proxy,
        },
    );
    let portal = linux_notification_proxy(&hub.connection)
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    let result: zbus::Result<()> = portal.call("AddNotification", &(id.as_str(), native));
    if let Err(error) = result {
        hub.unregister(&id);
        return Err(PlatformError::Platform(error.to_string().into()));
    }
    if let Some(evicted) = evicted {
        let _: zbus::Result<()> = portal.call("RemoveNotification", &(evicted.as_str()));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn read_bounded_notification_icon(path: &std::path::Path) -> Result<Vec<u8>, PlatformError> {
    use std::io::Read as _;

    let file = std::fs::File::open(path)
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    let mut bytes = Vec::new();
    file.take((crate::platform::MAX_SYSTEM_NOTIFICATION_ICON_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    if bytes.len() > crate::platform::MAX_SYSTEM_NOTIFICATION_ICON_BYTES {
        return Err(PlatformError::InvalidNotificationOptions);
    }
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn dismiss_linux_system_notification(tag: &str) -> Result<(), PlatformError> {
    let hub = linux_notification_hub()?;
    hub.unregister(tag);
    let portal = linux_notification_proxy(&hub.connection)
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    let result: zbus::Result<()> = portal.call("RemoveNotification", &(tag));
    result.map_err(|error| PlatformError::Platform(error.to_string().into()))
}

#[cfg(target_os = "linux")]
fn portable_notification_permission_status(
    _app_id: Option<&str>,
) -> Result<NotificationPermissionStatus, PlatformError> {
    linux_notification_hub().map(|_| NotificationPermissionStatus::Granted)
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn show_portable_system_notification(
    notification: SystemNotification,
    _proxy: EventLoopProxy<RuntimeEvent>,
    _app_id: Option<&str>,
) -> Result<(), PlatformError> {
    let status = std::process::Command::new("notify-send")
        .arg("--")
        .arg(notification.title.as_ref())
        .arg(notification.body.as_ref())
        .status()
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    if status.success() {
        Ok(())
    } else {
        Err(PlatformError::Platform(
            format!("notify-send exited with status {status}").into(),
        ))
    }
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn portable_notification_permission_status(
    _app_id: Option<&str>,
) -> Result<NotificationPermissionStatus, PlatformError> {
    Ok(NotificationPermissionStatus::Unsupported)
}

#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn finish_shell_request(
    responder: Option<crate::platform::PlatformResponder<()>>,
    result: Result<(), PlatformError>,
    action: &'static str,
) {
    if let Some(responder) = responder {
        responder.complete(result);
    } else if let Err(error) = result {
        tracing::warn!(%error, %action, "native shell action failed");
    }
}
