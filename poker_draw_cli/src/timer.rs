use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

// If secs == 0, waits indefinitely (no timeout)
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