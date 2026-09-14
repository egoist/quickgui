use std::{
    any::{Any, TypeId},
    cell::{Ref, RefMut},
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};

use bitflags::bitflags;
use thiserror::Error;

use crate::{
    AboutPanelOptions, Action, AnyAction, AppInfo, AppPaths, Assets, Color, CursorGrabMode,
    Display, DisplayId, Displays, ElementId, Entity, EntityId, EventEmitter, FileIconResponse,
    FileIconSize, FocusHandle, Font, Global, Image, KeyboardLayout, Menu, Point, Rect,
    RelaunchOptions, RelaunchRequest, Size, SystemInfo, SystemIntegrationError, SystemPreferences,
    TaskbarProgressState, UserTask, Vector, View, WindowHandle, WindowLevel, WindowOptions,
    WindowRegistry,
    clipboard::{ClipboardError, ClipboardItem, ClipboardService, ClipboardTarget},
    entity::{EntityEvent, MAX_ENTITY_EVENTS_PER_CALLBACK, MAX_ENTITY_NOTIFICATIONS_PER_EVENT},
    foreground::{AsyncViewContext, ForegroundTaskSpawnError, ForegroundTaskSpawner, Task},
    global::{GlobalStore, MAX_GLOBAL_NOTIFICATIONS_PER_EVENT},
    platform::{
        ActivationPolicy, ApplicationsFolderSupport, ColorPanelMode, DockAttention,
        DockAttentionRequest, MessageBoxOptions, MessageBoxResponseFuture,
        NotificationPermissionResponse, PathPromptOptions, PathPromptResponse, PlatformError,
        PlatformRequest, PlatformResponse, PromptButton, PromptLevel, SavePathOptions,
        SavePathResponse, ShareItem, ShellResponse, SystemNotification,
    },
    runtime::{
        MAX_SYSTEM_WINDOW_TABS, TrayIconOptions, WindowAppearance, WindowBackgroundAppearance,
        WindowCommand, WindowCommandError, WindowRequest, validate_taskbar_overlay_description,
        validate_taskbar_progress, validate_tray_options, validate_window_aspect_ratio,
        validate_window_bounds, validate_window_document_path, validate_window_opacity,
        validate_window_position, validate_window_size, validate_window_tabbing_identifier,
        validate_window_title,
    },
};

#[cfg(any(target_os = "macos", target_os = "windows"))]
use crate::menu::validate_menus;

mod context;
mod types;

pub use context::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use crate::{Global, IntoElement, ViewContext, div};

    use super::*;

    struct SecondaryView;

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[derive(Clone, Debug, Eq, PartialEq)]
    struct MenuTestAction;

    #[derive(Default)]
    struct TestGlobal(u32);

    impl Global for TestGlobal {}

    #[test]
    fn captured_pointer_localizes_window_coordinates_to_its_element() {
        let event = PointerEvent {
            size: Size::ZERO,
            phase: PointerPhase::Move,
            position: Point::new(342.0, 186.0),
            origin: Point::new(294.0, 168.0),
            local_position: Point::ZERO,
            local_origin: Point::ZERO,
            delta: Vector::new(6.0, 0.0),
            button: MouseButton::Left,
            modifiers: Modifiers::empty(),
        }
        .localize(Rect::new(286.0, 144.0, 640.0, 480.0));

        assert_eq!(event.position, Point::new(342.0, 186.0));
        assert_eq!(event.origin, Point::new(294.0, 168.0));
        assert_eq!(event.local_position, Point::new(56.0, 42.0));
        assert_eq!(event.local_origin, Point::new(8.0, 24.0));
        assert_eq!(event.delta, Vector::new(6.0, 0.0));
    }

    impl View for SecondaryView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div()
        }
    }

    #[test]
    fn window_commands_keep_stable_handles_and_options() {
        let parent = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(parent),
            ..EventContext::default()
        };
        let first = cx.open_window(
            WindowOptions::new("First").size(480.0, 320.0),
            SecondaryView,
        );
        let second = cx.open_window(WindowOptions::new("Second"), SecondaryView);

        assert_ne!(first, second);
        assert_eq!(cx.open_windows[0].handle, first);
        assert_eq!(cx.open_windows[0].options.title, "First");
        assert_eq!(cx.open_windows[0].options.size, Size::new(480.0, 320.0));
        assert_eq!(cx.open_windows[0].parent, Some(parent));
        assert_eq!(cx.open_windows[1].parent, Some(parent));
        cx.focus_window(first);
        cx.invalidate_window(second);
        cx.close_window_handle(first);
        cx.close_window();
        cx.prevent_close();
        assert_eq!(cx.focus_windows, [first]);
        assert_eq!(cx.invalidate_windows, [second]);
        assert_eq!(cx.close_windows, [first]);
        assert!(cx.close_current_window);
        assert!(cx.prevent_close);
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn native_menu_effects_retain_per_window_and_bounded_popup_intent() {
        let window = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(window),
            ..EventContext::default()
        };
        cx.set_window_menus([Menu::new("Window").item(crate::MenuItem::role(
            "Minimize",
            crate::OsAction::MinimizeWindow,
        ))]);
        assert!(matches!(cx.window_menus, Some(Some(ref menus)) if menus.len() == 1));
        cx.use_application_menus();
        assert!(matches!(cx.window_menus, Some(None)));

        assert_eq!(
            cx.show_native_popup_menu(Menu::new("Popup"), Some(Point::new(f32::NAN, 1.0))),
            Err(PlatformError::InvalidMenuPosition)
        );
        for index in 0..MAX_NATIVE_POPUP_MENUS_PER_EVENT {
            cx.show_native_popup_menu(
                Menu::new("Popup").action(format!("Action {index}"), MenuTestAction),
                None,
            )
            .unwrap();
        }
        assert_eq!(
            cx.show_native_popup_menu(Menu::new("Overflow"), None),
            Err(PlatformError::QueueFull)
        );
    }

    #[test]
    fn closing_popover_chain_restores_the_non_popover_owner_focus_once() {
        let owner = WindowHandle::next();
        let root = WindowHandle::next();
        let child = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(child),
            popover_owner_window: Some(owner),
            popover_root_window: Some(root),
            ..EventContext::default()
        };

        assert!(cx.close_popover_chain());
        assert!(cx.close_popover_chain());
        assert_eq!(cx.focus_windows, [owner]);
        assert_eq!(cx.close_windows, [root, root]);
        assert!(!cx.close_current_window);

        let mut root_context = EventContext {
            window: Some(root),
            popover_owner_window: Some(owner),
            popover_root_window: Some(root),
            ..EventContext::default()
        };
        assert!(root_context.close_popover_chain());
        assert_eq!(root_context.focus_windows, [owner]);
        assert!(root_context.close_current_window);

        let mut ordinary_window = EventContext::default();
        assert!(!ordinary_window.close_popover_chain());
        assert!(ordinary_window.focus_windows.is_empty());
    }

    #[test]
    fn relaunch_prepares_one_process_request_and_uses_orderly_exit() {
        let mut cx = EventContext::default();
        assert!(
            cx.relaunch_with(RelaunchOptions::new().executable("relative"))
                .is_err()
        );
        assert!(!cx.exit);
        assert!(cx.relaunch.is_none());

        cx.relaunch_with(
            RelaunchOptions::new()
                .executable(std::env::current_exe().unwrap())
                .without_arguments()
                .working_directory(std::env::current_dir().unwrap()),
        )
        .unwrap();
        assert!(cx.exit);
        assert!(cx.relaunch.is_some());
    }

    #[test]
    fn window_mutation_queue_is_bounded_and_validates_before_retaining() {
        let window = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(window),
            ..EventContext::default()
        };

        assert_eq!(
            cx.set_window_title("x".repeat(crate::MAX_WINDOW_TITLE_BYTES + 1)),
            Err(WindowCommandError::TitleTooLong)
        );
        assert!(cx.window_commands.is_empty());

        for _ in 0..crate::MAX_WINDOW_COMMANDS_PER_EVENT {
            assert!(cx.request_window_attention().is_ok());
        }
        assert_eq!(
            cx.request_window_attention(),
            Err(WindowCommandError::QueueFull)
        );
        assert_eq!(
            cx.window_commands.len(),
            crate::MAX_WINDOW_COMMANDS_PER_EVENT
        );
        assert!(
            cx.window_commands
                .iter()
                .all(|command| command.handle() == window)
        );
    }

    #[test]
    fn document_window_commands_validate_and_retain_exact_native_intent() {
        let window = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(window),
            ..EventContext::default()
        };

        assert_eq!(
            cx.set_document_path(PathBuf::new()),
            Err(WindowCommandError::InvalidDocumentPath)
        );
        assert_eq!(
            cx.set_tabbing_identifier(""),
            Err(WindowCommandError::InvalidTabbingIdentifier)
        );
        assert_eq!(
            cx.select_tab(MAX_SYSTEM_WINDOW_TABS),
            Err(WindowCommandError::InvalidTabIndex)
        );
        assert!(cx.window_commands.is_empty());

        cx.set_document_path("Cargo.toml").unwrap();
        cx.set_window_edited(true).unwrap();
        cx.show_character_palette().unwrap();
        cx.set_tabbing_identifier("dev.quickgui.workspace").unwrap();
        cx.select_next_tab().unwrap();
        cx.select_previous_tab().unwrap();
        cx.select_tab(7).unwrap();
        cx.merge_all_windows().unwrap();
        cx.move_tab_to_new_window().unwrap();
        cx.toggle_tab_bar().unwrap();
        cx.toggle_tab_overview().unwrap();
        cx.clear_represented_file().unwrap();
        cx.clear_tabbing_identifier().unwrap();

        assert_eq!(cx.window_commands.len(), 13);
        assert!(matches!(
            &cx.window_commands[0],
            WindowCommand::SetRepresentedFile(handle, Some(path))
                if *handle == window && path == &PathBuf::from("Cargo.toml")
        ));
        assert!(matches!(
            &cx.window_commands[1],
            WindowCommand::SetDocumentEdited(handle, true) if *handle == window
        ));
        assert!(matches!(
            &cx.window_commands[3],
            WindowCommand::SetTabbingIdentifier(handle, Some(identifier))
                if *handle == window && identifier == "dev.quickgui.workspace"
        ));
        assert!(matches!(
            &cx.window_commands[11],
            WindowCommand::SetRepresentedFile(handle, None) if *handle == window
        ));
        assert!(matches!(
            &cx.window_commands[12],
            WindowCommand::SetTabbingIdentifier(handle, None) if *handle == window
        ));
    }

    #[test]
    fn current_window_commands_fail_without_a_native_owner() {
        let mut cx = EventContext::default();

        assert_eq!(
            cx.resize_window(Size::new(640.0, 480.0)),
            Err(WindowCommandError::Unavailable)
        );
        assert!(cx.window_commands.is_empty());
    }

    #[test]
    fn platform_requests_are_bounded_and_validate_before_retaining() {
        let window = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(window),
            ..EventContext::default()
        };

        assert_eq!(cx.open_url(""), Err(PlatformError::InvalidUrl));
        assert!(cx.platform_requests.is_empty());

        for index in 0..crate::MAX_PLATFORM_REQUESTS_PER_EVENT {
            assert!(cx.open_url(format!("https://example.com/{index}")).is_ok());
        }
        assert_eq!(
            cx.open_url("https://example.com/overflow"),
            Err(PlatformError::QueueFull)
        );
        assert_eq!(
            cx.platform_requests.len(),
            crate::MAX_PLATFORM_REQUESTS_PER_EVENT
        );
    }

    #[test]
    fn native_dialogs_require_a_window_owner() {
        let mut cx = EventContext::default();
        assert!(matches!(
            cx.prompt(
                PromptLevel::Info,
                "Message",
                None,
                &[PromptButton::ok("OK")],
            ),
            Err(PlatformError::Unavailable)
        ));
        assert!(matches!(
            cx.prompt_for_paths(PathPromptOptions::new()),
            Err(PlatformError::Unavailable)
        ));
        assert!(cx.platform_requests.is_empty());
    }

    #[test]
    fn system_notifications_are_app_wide_but_still_bounded() {
        let mut cx = EventContext {
            app_info: Some(
                AppInfo::new("QuickGUI Test", "1.0.0", "dev.quickgui.test")
                    .expect("test application identity should be valid"),
            ),
            ..EventContext::default()
        };
        assert_eq!(
            cx.show_system_notification(SystemNotification::new("", "Title", "Body")),
            Err(PlatformError::InvalidNotificationTag)
        );
        assert!(cx.platform_requests.is_empty());

        assert!(
            cx.show_system_notification(SystemNotification::new(
                "background-job",
                "Finished",
                "The export is ready",
            ))
            .is_ok()
        );
        assert!(cx.dismiss_system_notification("background-job").is_ok());
        assert!(cx.notification_permission_status().is_ok());
        assert!(cx.request_notification_permission().is_ok());
        assert_eq!(cx.platform_requests.len(), 4);
        assert!(matches!(
            cx.platform_requests[2],
            PlatformRequest::NotificationPermissionStatus { .. }
        ));
        assert!(matches!(
            cx.platform_requests[3],
            PlatformRequest::RequestNotificationPermission { .. }
        ));
    }

    #[test]
    fn tray_icon_requests_validate_before_they_are_retained() {
        let icon = crate::TrayIconImage::from_rgba([255, 0, 0, 255], 1, 1).unwrap();
        let mut cx = EventContext::default();
        assert!(
            cx.set_tray_icon(crate::TrayIconOptions::new(0, icon.clone()))
                .is_err()
        );
        assert!(cx.platform_requests.is_empty());
        assert!(
            cx.set_tray_icon(crate::TrayIconOptions::new(1, icon))
                .is_ok()
        );
        assert!(cx.remove_tray_icon(1).is_ok());
        assert!(matches!(
            cx.platform_requests[0],
            PlatformRequest::SetTrayIcon(_)
        ));
        assert!(matches!(
            cx.platform_requests[1],
            PlatformRequest::RemoveTrayIcon(1)
        ));
    }

    #[test]
    fn event_context_globals_are_typed_and_changes_coalesce() {
        let mut cx = EventContext::default();
        assert!(!cx.has_global::<TestGlobal>());
        cx.set_global(TestGlobal(2));
        cx.update_global::<TestGlobal, _>(|global| global.0 += 3);
        cx.global_mut::<TestGlobal>().0 += 5;

        assert_eq!(cx.global::<TestGlobal>().0, 10);
        assert_eq!(cx.global_notifications, [TypeId::of::<TestGlobal>()]);
        assert!(!cx.notify_all_globals);
        assert_eq!(cx.remove_global::<TestGlobal>().0, 10);
        assert!(!cx.has_global::<TestGlobal>());
        assert_eq!(cx.global_notifications, [TypeId::of::<TestGlobal>()]);
    }

    #[test]
    fn too_many_global_changes_fall_back_to_notify_all() {
        let mut cx = EventContext {
            global_notifications: vec![
                TypeId::of::<TestGlobal>();
                MAX_GLOBAL_NOTIFICATIONS_PER_EVENT
            ],
            ..EventContext::default()
        };
        cx.note_global_changed(TypeId::of::<SecondaryView>());

        assert!(cx.global_notifications.is_empty());
        assert!(cx.notify_all_globals);
    }

    #[test]
    fn native_file_payloads_have_a_hard_path_count_bound() {
        let files = DroppedFiles::new(
            (0..=MAX_DROPPED_FILES).map(|index| PathBuf::from(format!("file-{index}"))),
        );

        assert_eq!(files.paths().len(), MAX_DROPPED_FILES);
        assert_eq!(files.paths().first(), Some(&PathBuf::from("file-0")));
        assert_eq!(
            files.paths().last(),
            Some(&PathBuf::from(format!("file-{}", MAX_DROPPED_FILES - 1)))
        );
        assert!(files.is_truncated());
    }

    #[test]
    fn outbound_file_payloads_bound_count_and_path_storage() {
        let files = FileDragPaths::new(
            (0..=MAX_EXTERNAL_DRAG_FILES)
                .map(|index| (PathBuf::from(format!("file-{index}")), index % 2 == 0)),
        );

        assert_eq!(files.entries().len(), MAX_EXTERNAL_DRAG_FILES);
        assert!(files.is_truncated());

        let oversized = PathBuf::from("x".repeat(MAX_EXTERNAL_DRAG_PATH_BYTES + 1));
        let files = FileDragPaths::new([
            (PathBuf::new(), false),
            (oversized, false),
            (PathBuf::from("kept.txt"), false),
        ]);
        assert_eq!(files.entries(), [(PathBuf::from("kept.txt"), false)]);
        assert!(files.is_truncated());

        assert_eq!(
            FileDragPaths::files([PathBuf::from("file.txt")]).entries(),
            [(PathBuf::from("file.txt"), false)]
        );
        assert_eq!(
            FileDragPaths::directories([PathBuf::from("folder")]).entries(),
            [(PathBuf::from("folder"), true)]
        );
    }

    #[test]
    fn outbound_text_and_url_payloads_are_utf8_safe_and_bounded() {
        let shared: Arc<str> = Arc::from("shared native drag text");
        let shared_text = ExternalDragText::new(Arc::clone(&shared));
        assert!(Arc::ptr_eq(&shared_text.text, &shared));

        let text =
            ExternalDragText::new(format!("{}é", "x".repeat(MAX_EXTERNAL_DRAG_TEXT_BYTES - 1)));
        assert_eq!(text.as_str().len(), MAX_EXTERNAL_DRAG_TEXT_BYTES - 1);
        assert!(text.is_truncated());
        assert!(text.as_str().is_char_boundary(text.as_str().len()));

        let shared_url: Arc<str> = Arc::from("https://example.com/路径?q=quickgui");
        let url = ExternalDragUrl::new(Arc::clone(&shared_url)).unwrap();
        assert!(Arc::ptr_eq(&url.0, &shared_url));
        assert_eq!(url.as_str(), "https://example.com/路径?q=quickgui");
        assert_eq!(
            ExternalDragUrl::new("relative/path").unwrap_err(),
            ExternalDragUrlError::InvalidScheme
        );
        assert_eq!(
            ExternalDragUrl::new("https://example.com/a b").unwrap_err(),
            ExternalDragUrlError::InvalidCharacter
        );
        assert!(matches!(
            ExternalDragUrl::new(format!(
                "https://example.com/{}",
                "x".repeat(MAX_EXTERNAL_DRAG_URL_BYTES)
            )),
            Err(ExternalDragUrlError::TooLarge { .. })
        ));
    }

    #[test]
    fn scroll_deltas_preserve_precision_and_bound_platform_values() {
        let pixels = ScrollDelta::Pixels(Vector::new(12.5, -24.0)).bounded();
        assert!(pixels.precise());
        assert_eq!(pixels.pixel_delta(40.0), Vector::new(12.5, -24.0));

        let lines = ScrollDelta::Lines(Vector::new(2.0, -3.0)).bounded();
        assert!(!lines.precise());
        assert_eq!(lines.pixel_delta(32.0), Vector::new(64.0, -96.0));

        assert_eq!(
            ScrollDelta::Pixels(Vector::new(f32::NAN, f32::INFINITY))
                .bounded()
                .pixel_delta(40.0),
            Vector::ZERO
        );
        assert_eq!(
            ScrollDelta::Lines(Vector::new(MAX_SCROLL_LINES_PER_EVENT * 2.0, -1.0))
                .bounded()
                .pixel_delta(f32::INFINITY),
            Vector::ZERO
        );
        assert_eq!(
            ScrollDelta::Pixels(Vector::new(MAX_SCROLL_PIXELS_PER_EVENT * 2.0, 0.0))
                .bounded()
                .pixel_delta(40.0),
            Vector::new(MAX_SCROLL_PIXELS_PER_EVENT, 0.0)
        );
    }

    #[test]
    fn input_propagation_and_default_prevention_are_independent() {
        let mut cx = EventContext::default();
        assert!(!cx.clear_text_selection);
        cx.clear_text_selection();
        assert!(cx.clear_text_selection);

        cx.stop_propagation();
        assert!(cx.stop_event_propagation);
        assert!(!cx.prevent_default);

        cx.prevent_default();
        assert!(cx.stop_event_propagation);
        assert!(cx.prevent_default);

        cx.propagate();
        assert!(!cx.stop_event_propagation);
        assert!(cx.prevent_default);
        assert!(cx.propagate_action);
    }

    #[test]
    fn raw_touch_samples_are_finite_and_pressure_bounded() {
        let event = TouchEvent {
            id: TouchId(42),
            phase: TouchPhase::Moved,
            position: Point::new(f32::INFINITY, -MAX_TOUCH_COORDINATE * 2.0),
            force: Some(1.5),
        }
        .bounded();

        assert_eq!(event.id, TouchId(42));
        assert_eq!(event.position, Point::new(0.0, -MAX_TOUCH_COORDINATE));
        assert_eq!(event.force, Some(1.0));
        assert_eq!(
            TouchEvent {
                force: Some(f32::NAN),
                ..TouchEvent::default()
            }
            .bounded()
            .force,
            Some(0.0)
        );
    }

    #[test]
    fn erased_command_registry_actions_keep_their_original_payload() {
        #[derive(Clone, Debug, Eq, PartialEq)]
        struct OpenLine(usize);

        let mut cx = EventContext::default();
        cx.dispatch_any_action(AnyAction::new(OpenLine(42)));

        assert_eq!(cx.actions.len(), 1);
        assert_eq!(
            cx.actions[0].downcast_ref::<OpenLine>(),
            Some(&OpenLine(42))
        );
    }

    #[test]
    fn cross_window_actions_are_parent_aware_and_hard_bounded() {
        #[derive(Clone, Debug, Eq, PartialEq)]
        struct OpenLine(usize);

        let parent = WindowHandle::next();
        let sibling = WindowHandle::next();
        let mut cx = EventContext {
            window: Some(WindowHandle::next()),
            parent_window: Some(parent),
            ..EventContext::default()
        };
        assert_eq!(cx.parent_window_handle(), Some(parent));
        assert!(cx.dispatch_action_to_parent(OpenLine(7)));
        assert!(cx.dispatch_action_to_window(sibling, OpenLine(9)));
        assert_eq!(cx.targeted_actions.len(), 2);
        assert_eq!(cx.targeted_actions[0].0, parent);
        assert_eq!(
            cx.targeted_actions[0].1.downcast_ref::<OpenLine>(),
            Some(&OpenLine(7))
        );
        assert_eq!(cx.targeted_actions[1].0, sibling);

        cx.targeted_actions.clear();
        for line in 0..MAX_TARGETED_ACTIONS_PER_EVENT {
            assert!(cx.dispatch_action_to_parent(OpenLine(line)));
        }
        assert!(!cx.dispatch_action_to_parent(OpenLine(usize::MAX)));
        assert_eq!(cx.targeted_actions.len(), MAX_TARGETED_ACTIONS_PER_EVENT);

        let mut root = EventContext::default();
        assert!(!root.dispatch_action_to_parent(OpenLine(1)));
        assert!(root.targeted_actions.is_empty());
    }

    #[test]
    fn programmatic_form_submissions_coalesce_and_stay_bounded() {
        let mut cx = EventContext::default();
        assert!(cx.submit_form("profile"));
        assert!(cx.submit_form("profile"));
        assert_eq!(cx.form_submissions, [ElementId::named("profile")]);

        for index in 1..MAX_FORM_SUBMISSIONS_PER_EVENT {
            assert!(cx.submit_form(index));
        }
        assert_eq!(cx.form_submissions.len(), MAX_FORM_SUBMISSIONS_PER_EVENT);
        assert!(!cx.submit_form(usize::MAX));
    }
}
