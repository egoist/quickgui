//! Bounded, cross-platform operating-system integrations independent of QuickGUI's renderer.

mod app_environment;
#[cfg(feature = "autostart")]
mod autostart;
#[cfg(feature = "crash-reporter")]
mod crash;
#[cfg(feature = "file-watcher")]
mod file_watcher;
#[cfg(feature = "file-watcher")]
pub use file_watcher::{FileWatchEvent, FileWatcher, MAX_WATCH_EVENT_PATHS, MAX_WATCH_ROOTS};
mod metrics;
mod power;
mod preferences;
mod process;
#[cfg(feature = "protocol")]
mod protocol;
#[cfg(feature = "secure-storage")]
mod secure_storage;

pub use app_environment::{
    AppInfo, AppPaths, MAX_APP_IDENTIFIER_BYTES, MAX_APP_NAME_BYTES, MAX_APP_VERSION_BYTES,
    MAX_PREFERRED_LANGUAGES, MAX_SYSTEM_LOCALE_BYTES, MAX_SYSTEM_LOCALES_TOTAL_BYTES,
    MAX_SYSTEM_TEXT_BYTES, OperatingSystem, OperatingSystemFamily, SystemBitness, SystemInfo,
};
#[cfg(feature = "autostart")]
pub use autostart::{AutoStart, AutoStartMode, AutoStartOptions};
#[cfg(feature = "crash-reporter")]
pub use crash::{
    BacktracePolicy, CRASH_REPORT_SCHEMA_VERSION, CrashKind, CrashLocation, CrashReport,
    CrashReporter, CrashReporterOptions, DEFAULT_MAX_CRASH_REPORT_BYTES, DEFAULT_MAX_CRASH_REPORTS,
    DEFAULT_WATCHDOG_HANG_THRESHOLD, DEFAULT_WATCHDOG_INTERVAL, MAX_CRASH_BACKTRACE_BYTES,
    MAX_CRASH_EXTRA_PARAMETERS, MAX_CRASH_MESSAGE_BYTES, MAX_CRASH_PARAMETER_KEY_BYTES,
    MAX_CRASH_PARAMETER_VALUE_BYTES, MAX_CRASH_REPORT_BYTES, MAX_CRASH_REPORTS,
    MAX_CRASH_UPLOAD_REPORTS, MAX_WATCHDOG_INTERVAL, MIN_WATCHDOG_INTERVAL, UploadSummary,
    Watchdog, WatchdogOptions,
};
pub use metrics::{
    CpuUsage, CpuUsageSampler, MAX_CPU_SAMPLE_INTERVAL, ProcessMetrics, SystemMemory,
};
pub use power::{
    BatteryState, BatteryStatus, IdleState, MAX_IDLE_THRESHOLD, MAX_POWER_ASSERTION_REASON_BYTES,
    PowerAssertion, PowerAssertionKind, PowerMonitor, PowerSource, PowerState, SessionState,
    ThermalState,
};
pub use preferences::{
    ColorScheme, PermissionKind, PermissionManager, PermissionStatus, SystemColor, SystemColorRole,
    SystemPreferences,
};
pub use process::{
    MAX_RELAUNCH_ARGUMENT_BYTES, MAX_RELAUNCH_ARGUMENTS, MAX_RELAUNCH_VALUE_BYTES, RelaunchOptions,
    RelaunchRequest, RelaunchedProcess,
};
#[cfg(feature = "protocol")]
pub use protocol::{ProtocolRegistration, ProtocolRegistrationOptions};
#[cfg(feature = "secure-storage")]
pub use secure_storage::SecureStorage;

use std::sync::Arc;

use thiserror::Error;

/// An operating-system integration rejected or failed an operation.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum SystemIntegrationError {
    #[error("the operating-system integration operation was cancelled")]
    Cancelled,
    #[error("this integration is not supported on the current operating system")]
    Unsupported,
    #[error("invalid system integration input: {0}")]
    InvalidInput(Arc<str>),
    #[error("operating-system integration failed: {0}")]
    Platform(Arc<str>),
    #[error("network operation failed: {0}")]
    Network(Arc<str>),
    #[error("update metadata is invalid: {0}")]
    InvalidUpdate(Arc<str>),
    #[error("update signature verification failed: {0}")]
    InvalidSignature(Arc<str>),
}

pub type Result<T> = std::result::Result<T, SystemIntegrationError>;

fn invalid(message: impl Into<Arc<str>>) -> SystemIntegrationError {
    SystemIntegrationError::InvalidInput(message.into())
}

fn platform(error: impl ToString) -> SystemIntegrationError {
    SystemIntegrationError::Platform(Arc::from(error.to_string()))
}

#[cfg(any(
    feature = "autostart",
    feature = "protocol",
    feature = "secure-storage"
))]
fn validate_text(value: &str, name: &str, maximum: usize) -> Result<()> {
    if value.is_empty() || value.contains('\0') || value.len() > maximum {
        return Err(invalid(format!(
            "{name} must be nonempty, NUL-free, and at most {maximum} UTF-8 bytes"
        )));
    }
    Ok(())
}
