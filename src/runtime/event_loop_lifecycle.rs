use super::*;

impl Runtime {
    pub(super) fn handle_resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.fatal_error.is_some() {
            return;
        }

        let first_ready = !self.ready;
        self.ready = true;
        if first_ready {
            // Rust apps and the Go/TypeScript host alike pump this runtime; announce the ready
            // socket the CLI waits on.
            notify_development_ready();
            // The install helper restores the previous version unless the new one gets this far.
            #[cfg(all(feature = "updater", not(target_arch = "wasm32")))]
            crate::updater::acknowledge_startup();
        }
        event_loop.set_control_flow(ControlFlow::Wait);
        self.initialize_global_shortcuts();
        self.refresh_displays(event_loop);
        self.invoke_finish_launching(event_loop);
        self.process_window_commands(event_loop);
        if let Some(urls) = self.pending_initial_open_urls.take() {
            self.invoke_open_urls(event_loop, urls);
        }
    }

    pub(super) fn handle_suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Desktop surfaces remain valid. Mobile surface teardown will be added with mobile shells.
    }

    pub(super) fn handle_exiting(&mut self, event_loop: &ActiveEventLoop) {
        // Native termination (for example macOS Quit) can bypass EventContext::exit. Route it
        // through the same child-first ownership teardown so foreground tasks and window-closed
        // callbacks never depend on which quit path the operating system selected.
        self.exit_requested = true;
        self.pending_windows.clear();
        self.close_requests
            .extend(self.window_handles.keys().copied());
        let closed = self.close_requested_window_trees(event_loop);
        self.invoke_window_closed_callbacks(event_loop, closed);
        #[cfg(target_os = "macos")]
        self.restore_native_tabbing_baseline(event_loop);
        self.pending_windows.clear();
        self.platform_requests.clear();
    }
}

fn notify_development_ready() {
    let Some(path) = std::env::var_os("QUICKGUI_READY_SOCKET") else {
        return;
    };
    #[cfg(unix)]
    {
        use std::io::Write as _;
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&path) {
            let _ = stream.write_all(b"ready\n");
        }
    }
    #[cfg(windows)]
    notify_windows_ready_socket(&path);
    #[cfg(not(any(unix, windows)))]
    {
        let _ = &path;
    }
}

/// Bun's `unix` listener is an AF_UNIX socket on Windows too, which std cannot connect to.
#[cfg(windows)]
fn notify_windows_ready_socket(path: &std::ffi::OsStr) {
    use windows_sys::Win32::Networking::WinSock::{
        AF_UNIX, INVALID_SOCKET, SOCK_STREAM, SOCKADDR, SOCKADDR_UN, WSACleanup, WSADATA,
        WSAStartup, closesocket, connect, send, socket,
    };

    let mut address = SOCKADDR_UN {
        sun_family: AF_UNIX,
        sun_path: [0; 108],
    };
    // Bun binds the UTF-8 path; the copy keeps a terminating NUL inside `sun_path`.
    let Some(path) = path.to_str() else {
        return;
    };
    if path.len() >= address.sun_path.len() || path.contains('\0') {
        return;
    }
    for (slot, byte) in address.sun_path.iter_mut().zip(path.bytes()) {
        *slot = byte as i8;
    }
    let message = b"ready\n";
    let mut data = WSADATA::default();
    // SAFETY: Winsock stays initialized until the balancing WSACleanup, the socket is closed before
    // it, and every pointer refers to a live local of the length passed with it.
    unsafe {
        if WSAStartup(0x0202, &mut data) != 0 {
            return;
        }
        let handle = socket(AF_UNIX.into(), SOCK_STREAM, 0);
        if handle != INVALID_SOCKET {
            let length = size_of::<SOCKADDR_UN>() as i32;
            if connect(handle, (&raw const address).cast::<SOCKADDR>(), length) == 0 {
                send(handle, message.as_ptr(), message.len() as i32, 0);
            }
            closesocket(handle);
        }
        WSACleanup();
    }
}
