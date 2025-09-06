use std::io::{self, Write};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(not(unix))]
use std::sync::mpsc;
#[cfg(not(unix))]
use std::thread;
#[cfg(not(unix))]
use std::time::{Duration, Instant};

#[cfg(unix)]
mod ffi {
    use std::os::unix::io::RawFd;

    #[repr(C)]
    pub struct PollFd {
        pub fd: RawFd,
        pub events: i16,
        pub revents: i16,
    }

    pub const POLLIN: i16 = 0x001;

    extern "C" {
        pub fn poll(fds: *mut PollFd, nfds: u64, timeout: i32) -> i32;
    }
}

// If secs == 0, waits indefinitely (no timeout)
#[cfg(unix)]
pub fn read_line_timeout(prompt: &str, secs: u64) -> Option<String> {
    print!("{prompt}");
    let _ = io::stdout().flush();

    let fd = io::stdin().as_raw_fd();
    let mut fds = [ffi::PollFd { fd, events: ffi::POLLIN, revents: 0 }];
    let timeout = if secs == 0 { -1 } else { (secs as i32) * 1000 };
    let res = unsafe { ffi::poll(fds.as_mut_ptr(), 1, timeout) };

    if res > 0 && (fds[0].revents & ffi::POLLIN) != 0 {
        let mut buf = String::new();
        if io::stdin().read_line(&mut buf).is_ok() {
            Some(buf)
        } else {
            None
        }
    } else if res == 0 {
        println!("\nTime out.");
        None
    } else {
        None
    }
}

#[cfg(not(unix))]
pub fn read_line_timeout(prompt: &str, secs: u64) -> Option<String> {
    print!("{prompt}");
    let _ = io::stdout().flush();

    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let mut buf = String::new();
        let _ = io::stdin().read_line(&mut buf);
        let _ = tx.send(buf);
    });

    if secs == 0 {
        return rx.recv().ok();
    }

    let start = Instant::now();
    loop {
        if let Ok(s) = rx.try_recv() {
            return Some(s);
        }
        if start.elapsed() >= Duration::from_secs(secs) {
            println!("\nTime out.");
            return None;
        }
        thread::sleep(Duration::from_millis(100));
    }
}
