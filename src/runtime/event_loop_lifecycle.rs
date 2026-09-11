use super::*;

impl Runtime {
    pub(super) fn handle_resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.fatal_error.is_some() {
            return;
        }

        let first_ready = !self.ready;
        self.ready = true;
        if first_ready {
            // Native Rust apps have no host C API; announce the same ready socket the CLI waits on.
            notify_development_ready();
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
    #[cfg(not(unix))]
    {
        let _ = &path;
    }
}
