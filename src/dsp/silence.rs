//! Redirect stderr to `/dev/null` around noisy Soapy / librtlsdr calls.

use std::fs::File;
use std::os::unix::io::AsRawFd;

pub struct SilencedStderr {
    backup: i32,
    _devnull: File,
}

impl SilencedStderr {
    pub fn new() -> Self {
        unsafe {
            let devnull = File::open("/dev/null").expect("/dev/null missing");
            let null_fd = devnull.as_raw_fd();
            let backup = libc::dup(2);
            libc::dup2(null_fd, 2);
            Self {
                backup,
                _devnull: devnull,
            }
        }
    }
}

impl Drop for SilencedStderr {
    fn drop(&mut self) {
        unsafe {
            libc::dup2(self.backup, 2);
            libc::close(self.backup);
        }
    }
}

pub fn silenced<R>(f: impl FnOnce() -> R) -> R {
    let _g = SilencedStderr::new();
    f()
}
