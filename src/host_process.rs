// The AppImage's AppRun points LD_LIBRARY_PATH, PATH, GTK_*, GST_*… at its own mount. A process
// started with them runs host programs against the bundled (older) libraries: the new AppImage's
// hooks die on the host bash ("undefined symbol: rl_trim_arg_from_keyseq" on Arch), a browser
// would load Ubuntu 22.04's GTK. So every entry under the mount is dropped; outside an AppImage
// the environment is passed through unchanged.
pub fn use_host_environment(cmd: &mut std::process::Command) {
    let appdir = std::env::var("APPDIR").unwrap_or_default();
    cmd.env_clear();
    for (key, value) in std::env::vars_os() {
        match value.to_str() {
            Some(text) if !appdir.is_empty() && text.contains(&appdir) => {
                let kept: Vec<&str> = text
                    .split(':')
                    .filter(|part| !part.is_empty() && !part.starts_with(&appdir))
                    .collect();
                if !kept.is_empty() {
                    cmd.env(key, kept.join(":"));
                }
            }
            _ => {
                cmd.env(key, value);
            }
        }
    }
    // WebKit and the AppImage runtime leave descriptors without CLOEXEC (one of them on the
    // mount), which would keep the runtime and its FUSE mount alive as long as the child runs.
    // SAFETY: only async-signal-safe syscalls between fork and exec.
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            if libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) != 0 {
                (3..1024).for_each(|fd| {
                    libc::close(fd);
                });
            }
            Ok(())
        });
    }
}

// Hands a URL to the desktop's default handler (xdg-open, else gio, …) as a host process.
pub fn open_url(url: &str) {
    use std::process::Stdio;

    for mut cmd in open::commands(url) {
        use_host_environment(&mut cmd);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        match cmd.spawn() {
            Ok(mut child) => {
                // xdg-open can wait for the browser it started; reap it off the main thread.
                std::thread::spawn(move || child.wait());
                return;
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                eprintln!("open {url}: {err}");
                return;
            }
        }
    }
    eprintln!("open {url}: no launcher found");
}
