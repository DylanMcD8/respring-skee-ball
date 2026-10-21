//! Same-directory replacement keeps the old file intact until writing succeeds.
use std::{fs::{self, File, OpenOptions}, io::{self, Write}, path::Path,
    sync::atomic::{AtomicU64, Ordering}};
static SERIAL: AtomicU64 = AtomicU64::new(0);

pub(crate) fn replace(path: &Path, data: &[u8]) -> io::Result<()> {
    replace_with(path, |file| { file.write_all(data)?; file.sync_all() })
}

fn replace_with(path: &Path, write: impl FnOnce(&mut File) -> io::Result<()>) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing parent"))?;
    let mut temporary = None;
    for _ in 0..32 {
        let candidate = parent.join(format!(".chuzzle-write-{}-{}", std::process::id(), SERIAL.fetch_add(1, Ordering::Relaxed)));
        match OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(file) => { temporary = Some((candidate, file)); break; }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let (temporary, mut file) = temporary.ok_or_else(|| io::Error::new(io::ErrorKind::AlreadyExists, "Temporary names exhausted"))?;
    let result = (|| {
        if let Ok(metadata) = fs::metadata(path) { file.set_permissions(metadata.permissions())?; }
        write(&mut file)?;
        fs::rename(&temporary, path)
    })();
    drop(file);
    if result.is_err() { let _ = fs::remove_file(&temporary); }
    // Directory sync is best effort: some host filesystems do not support it.
    if result.is_ok() { if let Ok(dir) = File::open(parent) { let _ = dir.sync_all(); } }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dir(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("chuzzle-atomic-{}-{name}", std::process::id()));
        fs::create_dir_all(&p).unwrap(); p
    }
    #[test] fn replaces_and_creates_complete_files() {
        let d=dir("success"); let p=d.join("save");
        replace(&p,b"old").unwrap(); replace(&p,b"complete new save").unwrap();
        assert_eq!(fs::read(&p).unwrap(),b"complete new save");
        replace(&p,b"").unwrap(); assert!(fs::read(&p).unwrap().is_empty());
        assert_eq!(fs::read_dir(&d).unwrap().count(),1); fs::remove_dir_all(d).unwrap();
    }
    #[test] fn partial_write_failure_preserves_previous_save() {
        let d=dir("failure"); let p=d.join("save"); fs::write(&p,b"good save").unwrap();
        let result=replace_with(&p,|f| { f.write_all(b"partial")?; Err(io::Error::other("simulated disk failure")) });
        assert!(result.is_err()); assert_eq!(fs::read(&p).unwrap(),b"good save");
        assert_eq!(fs::read_dir(&d).unwrap().count(),1); fs::remove_dir_all(d).unwrap();
    }
    #[test] fn rename_failure_cleans_temporary_and_preserves_destination() {
        let d=dir("rename"); let p=d.join("directory"); fs::create_dir(&p).unwrap();
        assert!(replace(&p,b"data").is_err()); assert!(p.is_dir());
        assert_eq!(fs::read_dir(&d).unwrap().count(),1); fs::remove_dir_all(d).unwrap();
    }
}
