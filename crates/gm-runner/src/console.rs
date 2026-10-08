//! The console window for stdout/stderr (show_debug_message output, errors,
//! runner info).
//!
//! On Windows the runner is a GUI-subsystem program (no console of its own), so
//! a standalone game started by double-click never shows one. Runs of a project
//! file from a command line get a console: the one of the parent process when
//! there is one (output lands in the calling cmd/PowerShell), else a new console
//! window (`new`, from `--new-console`, always creates one). The IDE starts the
//! runner with `--no-console` and its stdout/stderr piped into its Output Log, so
//! no console is ever attached or created then. Standard handles that are
//! redirected to a file or pipe (test harnesses, scripts) are kept as they are;
//! when both stdout and stderr are redirected no console is attached or created.
//! Elsewhere this does nothing.

/// Sets up the console (see the module docs).
#[cfg(windows)]
pub fn init(new: bool) {
    use windows::core::w;
    use windows::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, GetFileType, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TYPE_CHAR, FILE_TYPE_DISK,
        FILE_TYPE_PIPE, OPEN_EXISTING,
    };
    use windows::Win32::System::Console::{
        AllocConsole, AttachConsole, FreeConsole, GetConsoleMode, GetStdHandle, CONSOLE_MODE, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
        STD_HANDLE, STD_OUTPUT_HANDLE,
    };
    // SAFETY: plain Win32 calls on handles owned by this process.
    unsafe {
        // a file or pipe; also the NUL device (a character device that is not
        // a console), unless a new console is wanted
        let redirected = |h: STD_HANDLE| -> bool {
            match GetStdHandle(h) {
                Ok(x) if !x.is_invalid() && !x.0.is_null() => match GetFileType(x) {
                    t if t == FILE_TYPE_DISK || t == FILE_TYPE_PIPE => true,
                    t if t == FILE_TYPE_CHAR && !new => {
                        let mut mode = CONSOLE_MODE(0);
                        GetConsoleMode(x, &mut mode).is_err()
                    }
                    _ => false,
                },
                _ => false,
            }
        };
        let (out_r, err_r) = (redirected(STD_OUTPUT_HANDLE), redirected(STD_ERROR_HANDLE));
        if out_r && err_r {
            return;
        }
        let attached = if new {
            let _ = FreeConsole();
            AllocConsole().is_ok()
        } else {
            AttachConsole(ATTACH_PARENT_PROCESS).is_ok() || AllocConsole().is_ok()
        };
        if !attached {
            return;
        }
        let conout = CreateFileW(
            w!("CONOUT$"),
            (GENERIC_READ | GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        );
        let Ok(conout): Result<HANDLE, _> = conout else { return };
        if !out_r {
            let _ = SetStdHandle(STD_OUTPUT_HANDLE, conout);
        }
        if !err_r {
            let _ = SetStdHandle(STD_ERROR_HANDLE, conout);
        }
    }
}

#[cfg(not(windows))]
pub fn init(_new: bool) {}
