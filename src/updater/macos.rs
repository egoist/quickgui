//! Sparkle routing driver adapted from Waku: scheduled results stay in the app; explicit
//! checks retain Sparkle's standard UI. Every Objective-C access runs on the native main queue.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// The core still builds against objc2 0.5, so both crates name this generation explicitly.
use block2_06::{DynBlock, RcBlock};
use objc2_06::rc::Retained;
use objc2_06::runtime::{AnyClass, AnyObject, NSObject, NSObjectProtocol};
use objc2_06::{
    ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel,
};
use objc2_foundation_03::NSString;

use super::{
    Command, Result, SessionEvents as Events, SessionOptions as Options, Sink, UpdateStatus,
    UpdaterEvent,
};
use std::sync::Arc;

const USER_UPDATE_CHOICE_INSTALL: isize = 1;
const UPDATE_CHECK_USER_INITIATED: isize = 0;
const UPDATE_CHECK_IN_BACKGROUND: isize = 1;
const MANUAL_CHECK_MAX_RETRIES: u16 = 200;

// objc2 generates additional unsafe marker traits without inheriting the protocol docs.
#[allow(clippy::missing_safety_doc)]
mod protocols {
    use objc2_06::{extern_protocol, runtime::NSObjectProtocol};
    extern_protocol!(
        /// Dynamically loaded from the embedded Sparkle framework.
        ///
        /// # Safety
        /// Implementors must use Sparkle's exact Objective-C selectors and block signatures.
        pub(super) unsafe trait SPUUserDriver: NSObjectProtocol {}
    );

    extern_protocol!(
        /// Only the update-cycle completion callback is implemented below.
        ///
        /// # Safety
        /// Implementors must conform to SPUUpdaterDelegate on the native main thread.
        pub(super) unsafe trait SPUUpdaterDelegate: NSObjectProtocol {}
    );
}
use protocols::{SPUUpdaterDelegate, SPUUserDriver};

struct PendingUpdate {
    appcast_item: Retained<AnyObject>,
    state: Retained<AnyObject>,
    reply: RcBlock<dyn Fn(isize)>,
}

struct UserDriverIvars {
    /// Explicit checks and the one-time automatic-check permission prompt
    /// use Sparkle's own windows. Scheduled checks stay inside the application.
    standard_driver: Retained<AnyObject>,
    standard_presentation: Cell<bool>,
    standard_update_check: Cell<Option<isize>>,
    manual_check_requested: Rc<Cell<bool>>,
    manual_check_retry_count: Cell<u16>,
    pending_update: RefCell<Option<PendingUpdate>>,
    cancellation: RefCell<Option<RcBlock<dyn Fn()>>>,
    status: Rc<Cell<UpdateStatus>>,
    events: Events,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "QuickGuiSparkleUserDriver"]
    #[thread_kind = MainThreadOnly]
    #[ivars = UserDriverIvars]
    struct UserDriver;

    unsafe impl SPUUserDriver for UserDriver {
        #[unsafe(method(showUpdatePermissionRequest:reply:))]
        fn show_update_permission_request(
            &self,
            request: &AnyObject,
            reply: &DynBlock<dyn Fn(*mut AnyObject)>,
        ) {
            let _: () = unsafe {
                msg_send![
                    &*self.ivars().standard_driver,
                    showUpdatePermissionRequest: request,
                    reply: reply
                ]
            };
        }

        #[unsafe(method(showUserInitiatedUpdateCheckWithCancellation:))]
        fn show_user_initiated_update_check(&self, cancellation: &DynBlock<dyn Fn()>) {
            self.ivars().cancellation.replace(Some(cancellation.copy()));
            self.begin_standard_presentation(UPDATE_CHECK_USER_INITIATED);
            let _: () = unsafe {
                msg_send![
                    &*self.ivars().standard_driver,
                    showUserInitiatedUpdateCheckWithCancellation: cancellation
                ]
            };
        }

        #[unsafe(method(showUpdateFoundWithAppcastItem:state:reply:))]
        fn show_update_found(
            &self,
            appcast_item: &AnyObject,
            state: &AnyObject,
            reply: &DynBlock<dyn Fn(isize)>,
        ) {
            let version: *mut NSString = unsafe { msg_send![appcast_item, displayVersionString] };
            let version = unsafe { version.as_ref() }.map(ToString::to_string).unwrap_or_default();
            self.ivars().events.update("state", |e| { e.version = version.chars().take(128).collect(); });
            if self.ivars().manual_check_requested.replace(false) {
                self.begin_standard_presentation(UPDATE_CHECK_IN_BACKGROUND);
                self.show_update_found_with_standard_driver(appcast_item, state, reply);
                return;
            }
            if self.uses_standard_presentation() {
                self.show_update_found_with_standard_driver(appcast_item, state, reply);
                return;
            }

            let appcast_item = unsafe {
                Retained::retain(std::ptr::from_ref(appcast_item).cast_mut())
                    .expect("Sparkle supplied a non-null appcast item")
            };
            let state = unsafe {
                Retained::retain(std::ptr::from_ref(state).cast_mut())
                    .expect("Sparkle supplied a non-null update state")
            };
            self.ivars().pending_update.replace(Some(PendingUpdate {
                appcast_item,
                state,
                reply: reply.copy(),
            }));
            self.set_status(UpdateStatus::Available);
        }

        #[unsafe(method(showUpdateReleaseNotesWithDownloadData:))]
        fn show_update_release_notes(&self, download_data: &AnyObject) {
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showUpdateReleaseNotesWithDownloadData: download_data
                    ]
                };
            }
        }

        #[unsafe(method(showUpdateReleaseNotesFailedToDownloadWithError:))]
        fn show_update_release_notes_failed(&self, error: &AnyObject) {
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showUpdateReleaseNotesFailedToDownloadWithError: error
                    ]
                };
            }
        }

        #[unsafe(method(showUpdateNotFoundWithError:acknowledgement:))]
        fn show_update_not_found(
            &self,
            error: &AnyObject,
            acknowledgement: &DynBlock<dyn Fn()>,
        ) {
            if self.ivars().manual_check_requested.replace(false) {
                self.begin_standard_presentation(UPDATE_CHECK_IN_BACKGROUND);
            }
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showUpdateNotFoundWithError: error,
                        acknowledgement: acknowledgement
                    ]
                };
                return;
            }

            self.clear_update();
            self.send(UpdaterEvent::UpToDate);
            acknowledgement.call(());
        }

        #[unsafe(method(showUpdaterError:acknowledgement:))]
        fn show_updater_error(
            &self,
            error: &AnyObject,
            acknowledgement: &DynBlock<dyn Fn()>,
        ) {
            if self.ivars().manual_check_requested.replace(false) {
                self.begin_standard_presentation(UPDATE_CHECK_IN_BACKGROUND);
            }
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showUpdaterError: error,
                        acknowledgement: acknowledgement
                    ]
                };
                return;
            }

            self.clear_update();
            self.send(UpdaterEvent::Failed(error_description(error)));
            acknowledgement.call(());
        }

        #[unsafe(method(showDownloadInitiatedWithCancellation:))]
        fn show_download_initiated(&self, cancellation: &DynBlock<dyn Fn()>) {
            self.ivars().cancellation.replace(Some(cancellation.copy()));
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showDownloadInitiatedWithCancellation: cancellation
                    ]
                };
                return;
            }
            self.set_status(UpdateStatus::Installing);
        }

        #[unsafe(method(showDownloadDidReceiveExpectedContentLength:))]
        fn show_expected_content_length(&self, expected_content_length: u64) {
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showDownloadDidReceiveExpectedContentLength: expected_content_length
                    ]
                };
            }
        }

        #[unsafe(method(showDownloadDidReceiveDataOfLength:))]
        fn show_downloaded_data(&self, length: u64) {
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showDownloadDidReceiveDataOfLength: length
                    ]
                };
            }
        }

        #[unsafe(method(showDownloadDidStartExtractingUpdate))]
        fn show_extracting_update(&self) {
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![&*self.ivars().standard_driver, showDownloadDidStartExtractingUpdate]
                };
                return;
            }
            self.set_status(UpdateStatus::Installing);
        }

        #[unsafe(method(showExtractionReceivedProgress:))]
        fn show_extraction_progress(&self, progress: f64) {
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showExtractionReceivedProgress: progress
                    ]
                };
            }
        }

        #[unsafe(method(showReadyToInstallAndRelaunch:))]
        fn show_ready_to_install(&self, reply: &DynBlock<dyn Fn(isize)>) {
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showReadyToInstallAndRelaunch: reply
                    ]
                };
                return;
            }
            self.set_status(UpdateStatus::Installing);
            reply.call((USER_UPDATE_CHOICE_INSTALL,));
        }

        #[unsafe(method(showInstallingUpdateWithApplicationTerminated:retryTerminatingApplication:))]
        fn show_installing_update(
            &self,
            application_terminated: bool,
            retry_terminating_application: &DynBlock<dyn Fn()>,
        ) {
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showInstallingUpdateWithApplicationTerminated: application_terminated,
                        retryTerminatingApplication: retry_terminating_application
                    ]
                };
                return;
            }
            self.set_status(UpdateStatus::Installing);
        }

        #[unsafe(method(showUpdateInstalledAndRelaunched:acknowledgement:))]
        fn show_update_installed(
            &self,
            relaunched: bool,
            acknowledgement: &DynBlock<dyn Fn()>,
        ) {
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![
                        &*self.ivars().standard_driver,
                        showUpdateInstalledAndRelaunched: relaunched,
                        acknowledgement: acknowledgement
                    ]
                };
                return;
            }
            acknowledgement.call(());
        }

        #[unsafe(method(dismissUpdateInstallation))]
        fn dismiss_update_installation(&self) {
            if self.uses_standard_presentation() {
                let _: () = unsafe {
                    msg_send![&*self.ivars().standard_driver, dismissUpdateInstallation]
                };
                self.ivars().standard_presentation.set(false);
                self.ivars().standard_update_check.set(None);
            }
            self.clear_update();
        }

        #[unsafe(method(showUpdateInFocus))]
        fn show_update_in_focus(&self) {
            if !self.uses_standard_presentation() && self.present_pending_update() {
                return;
            }
            let _: () = unsafe {
                msg_send![&*self.ivars().standard_driver, showUpdateInFocus]
            };
        }
    }

    impl UserDriver {
        #[unsafe(method(startRequestedStandardUpdateCheck:))]
        fn start_requested_standard_update_check(&self, updater: &AnyObject) {
            if !self.ivars().manual_check_requested.get() {
                return;
            }
            if self.present_pending_update() {
                self.ivars().manual_check_requested.set(false);
                return;
            }

            let can_check: bool = unsafe { msg_send![updater, canCheckForUpdates] };
            if can_check {
                self.ivars().manual_check_requested.set(false);
                self.begin_standard_presentation(UPDATE_CHECK_USER_INITIATED);
                let _: () = unsafe { msg_send![updater, checkForUpdates] };
            } else if self.ivars().manual_check_retry_count.get()
                < MANUAL_CHECK_MAX_RETRIES
            {
                self.ivars()
                    .manual_check_retry_count
                    .set(self.ivars().manual_check_retry_count.get() + 1);
                self.schedule_requested_standard_check(updater, 0.05);
            } else {
                self.ivars().manual_check_requested.set(false);
                let _: () = unsafe {
                    msg_send![&*self.ivars().standard_driver, dismissUpdateInstallation]
                };
            }
        }
    }

    unsafe impl SPUUpdaterDelegate for UserDriver {
        #[unsafe(method(updater:didFinishUpdateCycleForUpdateCheck:error:))]
        fn did_finish_update_cycle(
            &self,
            _updater: &AnyObject,
            update_check: isize,
            _error: Option<&AnyObject>,
        ) {
            if self.ivars().standard_update_check.get() == Some(update_check) {
                self.ivars().standard_presentation.set(false);
                self.ivars().standard_update_check.set(None);
            }
        }
    }

    unsafe impl NSObjectProtocol for UserDriver {}
);

impl UserDriver {
    fn new(
        mtm: MainThreadMarker,
        standard_driver: Retained<AnyObject>,
        status: Rc<Cell<UpdateStatus>>,
        events: Events,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(UserDriverIvars {
            standard_driver,
            standard_presentation: Cell::new(false),
            standard_update_check: Cell::new(None),
            manual_check_requested: Rc::new(Cell::new(false)),
            manual_check_retry_count: Cell::new(0),
            pending_update: RefCell::new(None),
            cancellation: RefCell::new(None),
            status,
            events,
        });
        unsafe { msg_send![super(this), init] }
    }

    fn uses_standard_presentation(&self) -> bool {
        self.ivars().standard_presentation.get()
    }

    fn begin_standard_presentation(&self, update_check: isize) {
        self.ivars().standard_presentation.set(true);
        self.ivars().standard_update_check.set(Some(update_check));
    }

    fn schedule_requested_standard_check(&self, updater: &AnyObject, delay: f64) {
        let _: () = unsafe {
            msg_send![
                self,
                performSelector: sel!(startRequestedStandardUpdateCheck:),
                withObject: updater,
                afterDelay: delay
            ]
        };
    }

    fn show_update_found_with_standard_driver(
        &self,
        appcast_item: &AnyObject,
        state: &AnyObject,
        reply: &DynBlock<dyn Fn(isize)>,
    ) {
        let _: () = unsafe {
            msg_send![
                &*self.ivars().standard_driver,
                showUpdateFoundWithAppcastItem: appcast_item,
                state: state,
                reply: reply
            ]
        };
        // A result discovered by the silent checker still has
        // `state.userInitiated == false`, so the standard driver may apply
        // its gentle-reminder rules and leave the alert hidden. The menu
        // action is explicit, so bring the newly created alert forward.
        let _: () = unsafe { msg_send![&*self.ivars().standard_driver, showUpdateInFocus] };
    }

    /// Promote the result already held by a silent automatic check into
    /// Sparkle's standard updater window without discarding it and
    /// starting a second network request.
    fn present_pending_update(&self) -> bool {
        let Some(update) = self.ivars().pending_update.borrow_mut().take() else {
            return false;
        };

        self.begin_standard_presentation(UPDATE_CHECK_IN_BACKGROUND);
        self.set_status(UpdateStatus::Idle);
        self.show_update_found_with_standard_driver(
            &update.appcast_item,
            &update.state,
            &update.reply,
        );
        true
    }

    /// Show Sparkle's standard checking UI immediately, then start a real
    /// user-initiated check as soon as any silent automatic session has
    /// finished tearing down.
    fn request_standard_check(&self, updater: &AnyObject) -> bool {
        if self.present_pending_update() {
            return true;
        }
        if self.uses_standard_presentation() {
            let _: () = unsafe { msg_send![updater, checkForUpdates] };
            return true;
        }
        if self.ivars().manual_check_requested.get() {
            let _: () = unsafe { msg_send![&*self.ivars().standard_driver, showUpdateInFocus] };
            return true;
        }
        if self.ivars().status.get() == UpdateStatus::Installing {
            return false;
        }

        let can_check: bool = unsafe { msg_send![updater, canCheckForUpdates] };
        if can_check {
            self.begin_standard_presentation(UPDATE_CHECK_USER_INITIATED);
            let _: () = unsafe { msg_send![updater, checkForUpdates] };
            return true;
        }

        self.ivars().manual_check_requested.set(true);
        self.ivars().manual_check_retry_count.set(0);
        let manual_check_requested = self.ivars().manual_check_requested.clone();
        let cancellation: RcBlock<dyn Fn()> = RcBlock::new(move || {
            manual_check_requested.set(false);
        });
        let _: () = unsafe {
            msg_send![
                &*self.ivars().standard_driver,
                showUserInitiatedUpdateCheckWithCancellation: &*cancellation
            ]
        };

        self.schedule_requested_standard_check(updater, 0.0);
        true
    }

    fn send(&self, event: UpdaterEvent) {
        self.ivars().events.try_send(event);
    }

    fn set_status(&self, status: UpdateStatus) {
        if self.ivars().status.replace(status) != status {
            self.send(UpdaterEvent::StatusChanged(status));
        }
    }

    fn clear_update(&self) {
        self.ivars().pending_update.borrow_mut().take();
        self.set_status(UpdateStatus::Idle);
    }

    fn install_available_update(&self) -> bool {
        if self.uses_standard_presentation() {
            return false;
        }
        let Some(update) = self.ivars().pending_update.borrow_mut().take() else {
            return false;
        };
        self.set_status(UpdateStatus::Installing);
        update.reply.call((USER_UPDATE_CHOICE_INSTALL,));
        true
    }
}

struct Updater {
    updater: Retained<AnyObject>,
    driver: Retained<UserDriver>,
}
struct Session {
    id: u32,
    events: Events,
    updater: Option<Updater>,
}
thread_local! { static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) }; }

pub(super) fn invoke(id: u32, method: &str, params: &str, sink: Arc<Sink>) -> Result<()> {
    if method == "start" {
        let options: Options = serde_json::from_str(params).map_err(|e| e.to_string())?;
        options.validate()?;
        return SESSION.with(|slot| {
            if slot.borrow().is_some() {
                return Err("an updater is already running; share one per application".into());
            }
            let events = Events::new(sink.clone(), options.automatic_checks);
            let updater = if options.disabled() {
                None
            } else {
                Some(Updater::new(&options, events.clone())?)
            };
            events.update("state", |e| {
                e.status = if updater.is_some() {
                    UpdateStatus::Idle
                } else {
                    UpdateStatus::Disabled
                }
            });
            *slot.borrow_mut() = Some(Session {
                id,
                events,
                updater,
            });
            sink.reply();
            Ok(())
        });
    }
    let command: Command = serde_json::from_str(params).map_err(|e| e.to_string())?;
    SESSION.with(|slot| {
        let mut slot=slot.borrow_mut();
        let session=slot.as_mut().filter(|s|s.id==command.session).ok_or("updater session has been closed")?;
        if method=="stop" { session.events.close(); slot.take(); sink.reply(); return Ok(()); }
        let updater=session.updater.as_ref().ok_or("updating is disabled in development builds")?;
        match method {
            "check"=>{ updater.driver.request_standard_check(&updater.updater); }
            "install"=>{ if !updater.driver.install_available_update() { return Err("no pending automatic update; use Check for Updates for Sparkle's active window".into()); } }
            "automatic"=>{
                let enabled=command.value.as_bool().ok_or("automatic expects a boolean")?;
                unsafe { let _:()=msg_send![&*updater.updater,setAutomaticallyChecksForUpdates:enabled]; }
                session.events.update("state",|e|e.automatic_checks=enabled);
                if enabled { unsafe { let _:()=msg_send![&*updater.updater,checkForUpdatesInBackground]; } }
            }
            _=>return Err(format!("unknown updater command: {method}")),
        }
        sink.reply(); Ok(())
    })
}

#[allow(dead_code)] // See `super::shutdown`.
pub(super) fn shutdown() {
    SESSION.with(|slot| {
        if let Some(session) = slot.borrow_mut().take() {
            session.events.close();
        }
    });
}

impl Updater {
    fn new(options: &Options, events: Events) -> Result<Self> {
        let mtm = MainThreadMarker::new()
            .ok_or("Sparkle must be initialized on the native main thread")?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let library = executable
            .parent()
            .and_then(|p| p.parent())
            .ok_or("updating requires a macOS app bundle")?
            .join("Frameworks/Sparkle.framework/Sparkle");
        let library =
            std::ffi::CString::new(std::os::unix::ffi::OsStrExt::as_bytes(library.as_os_str()))
                .map_err(|e| e.to_string())?;
        // Keep the framework loaded for process lifetime, like the service extension itself.
        if unsafe { libc::dlopen(library.as_ptr(), libc::RTLD_NOW) }.is_null() {
            return Err("could not load embedded Sparkle.framework; rebuild the app with the updater extension".into());
        }
        let bundle_class = AnyClass::get(c"NSBundle").ok_or("NSBundle unavailable")?;
        let updater_class = AnyClass::get(c"SPUUpdater").ok_or("SPUUpdater unavailable")?;
        let driver_class =
            AnyClass::get(c"SPUStandardUserDriver").ok_or("SPUStandardUserDriver unavailable")?;
        let bundle: *mut AnyObject = unsafe { msg_send![bundle_class, mainBundle] };
        if bundle.is_null() {
            return Err("updating requires an app bundle".into());
        }
        let key = NSString::from_str("SUPublicEDKey");
        let embedded: *mut NSString = unsafe { msg_send![bundle,objectForInfoDictionaryKey:&*key] };
        if unsafe { embedded.as_ref() }
            .map(ToString::to_string)
            .as_deref()
            != Some(options.public_key.as_str())
        {
            return Err(
                "updater publicKey must match SUPublicEDKey embedded by [updates] in quickgui.toml"
                    .into(),
            );
        }
        let standard: Retained<AnyObject> = unsafe {
            let allocated: *mut AnyObject = msg_send![driver_class, alloc];
            let initialized: *mut AnyObject = msg_send![allocated,initWithHostBundle:bundle,delegate:std::ptr::null_mut::<AnyObject>()];
            Retained::from_raw(initialized).ok_or("could not create Sparkle user driver")?
        };
        let driver = UserDriver::new(
            mtm,
            standard,
            Rc::new(Cell::new(UpdateStatus::Idle)),
            events.clone(),
        );
        let updater: Retained<AnyObject> = unsafe {
            let allocated: *mut AnyObject = msg_send![updater_class, alloc];
            let initialized: *mut AnyObject = msg_send![allocated,initWithHostBundle:bundle,applicationBundle:bundle,userDriver:&*driver,delegate:&*driver];
            Retained::from_raw(initialized).ok_or("could not initialize Sparkle")?
        };
        let feed = NSString::from_str(&options.feed_url);
        let url_class = AnyClass::get(c"NSURL").ok_or("NSURL unavailable")?;
        let url: *mut AnyObject = unsafe { msg_send![url_class,URLWithString:&*feed] };
        // Feed override only changes the running session; key and default preferences remain
        // application metadata. Sparkle persists user choices in the application's defaults.
        unsafe {
            let _: () = msg_send![&*updater,setFeedURL:url];
        }
        let mut error: *mut AnyObject = std::ptr::null_mut();
        let started: bool = unsafe { msg_send![&*updater,startUpdater:&mut error] };
        if !started {
            return Err(unsafe { error.as_ref() }
                .map(error_description)
                .unwrap_or_else(|| "Sparkle rejected the updater configuration".into()));
        }
        let automatic: bool = unsafe { msg_send![&*updater, automaticallyChecksForUpdates] };
        events.update("state", |e| e.automatic_checks = automatic);
        if automatic {
            unsafe {
                let _: () = msg_send![&*updater, checkForUpdatesInBackground];
            }
        }
        Ok(Self { updater, driver })
    }
}
impl Drop for Updater {
    fn drop(&mut self) {
        self.driver.ivars().manual_check_requested.set(false);
        unsafe {
            let _: () =
                msg_send![NSObject::class(),cancelPreviousPerformRequestsWithTarget:&*self.driver];
        }
        let cancel = self.driver.ivars().cancellation.borrow_mut().take();
        if let Some(cancel) = cancel {
            cancel.call(());
        }
        let pending = self.driver.ivars().pending_update.borrow_mut().take();
        if let Some(pending) = pending {
            pending.reply.call((2,));
        } // dismiss, never skip a version
        unsafe {
            let _: () = msg_send![
                &*self.driver.ivars().standard_driver,
                dismissUpdateInstallation
            ];
        }
    }
}
fn error_description(error: &AnyObject) -> String {
    let description: *mut NSString = unsafe { msg_send![error, localizedDescription] };
    unsafe { description.as_ref() }
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown Sparkle error".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routing_protocols_match_the_staged_sparkle_framework() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../extensions/updater/lib/darwin-arm64/Sparkle.framework/Sparkle");
        if !path.exists() {
            return;
        }
        let path = std::ffi::CString::new(std::os::unix::ffi::OsStrExt::as_bytes(path.as_os_str()))
            .unwrap();
        assert!(!unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW) }.is_null());
        let _ = UserDriver::class();
    }
}
