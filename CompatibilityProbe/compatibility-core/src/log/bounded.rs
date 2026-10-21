//! Best-effort bounded diagnostic storage; independent of the runtime.
use std::{fs::{self, File}, io::Write, path::PathBuf};

pub struct BoundedLog { path: PathBuf, file: Option<File>, bytes: usize, limit: usize }
impl BoundedLog {
    pub fn new(path: PathBuf, limit: usize) -> Self {
        // Preserve the previous session before opening the next one.
        let previous = path.with_extension("previous.log");
        let can_open = !path.exists() || fs::rename(&path, &previous).is_ok();
        if can_open {
            if let Ok(file) = fs::OpenOptions::new().write(true).open(&previous) {
                if file.metadata().map(|m| m.len() > limit as u64).unwrap_or(false) {
                    let _ = file.set_len(limit as u64);
                }
            }
        }
        let file = if can_open { File::create(&path).ok() } else { None };
        Self { path, file, bytes: 0, limit }
    }
    pub fn line(&mut self, line: &str) {
        if self.file.is_none() || self.limit < 2 { return; }
        let mut end = line.len().min(self.limit - 1);
        while !line.is_char_boundary(end) { end -= 1; }
        let data = &line.as_bytes()[..end];
        if self.bytes + data.len() + 1 > self.limit {
            self.file = None;
            if fs::rename(&self.path, self.path.with_extension("previous.log")).is_err() { return; }
            self.file = File::create(&self.path).ok();
            self.bytes = 0;
        }
        if let Some(file) = self.file.as_mut() {
            if file.write_all(data).and_then(|_| file.write_all(b"\n")).is_err() {
                self.file = None;
            } else { self.bytes += data.len() + 1; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn directory(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("chuzzle-log-{}-{}", std::process::id(), label));
        fs::create_dir_all(&p).unwrap(); p
    }
    #[test] fn unavailable_directory_is_nonfatal() {
        let d = directory("missing");
        let mut log = BoundedLog::new(d.join("absent/log.txt"), 32);
        log.line("still running"); assert!(log.file.is_none());
        fs::remove_dir_all(d).unwrap();
    }
    #[test] fn rotation_caps_both_files_and_keeps_recent_lines() {
        let d = directory("rotate"); let p = d.join("log.txt");
        let mut log = BoundedLog::new(p.clone(), 32);
        for i in 0..100 { log.line(&format!("line {i}")); }
        drop(log);
        assert!(fs::metadata(&p).unwrap().len() <= 32);
        assert!(fs::metadata(p.with_extension("previous.log")).unwrap().len() <= 32);
        assert!(fs::read_to_string(&p).unwrap().contains("line 99"));
        let log = BoundedLog::new(p.clone(), 32); drop(log);
        assert!(fs::read_to_string(p.with_extension("previous.log")).unwrap().contains("line 99"));
        fs::remove_dir_all(d).unwrap();
    }
    #[test] fn oversized_unicode_message_stays_valid_and_bounded() {
        let d = directory("unicode"); let p = d.join("log.txt");
        let mut log = BoundedLog::new(p.clone(), 8); log.line("🎮🎮🎮"); drop(log);
        assert_eq!(fs::read_to_string(&p).unwrap(), "🎮\n");
        fs::remove_dir_all(d).unwrap();
    }
}
