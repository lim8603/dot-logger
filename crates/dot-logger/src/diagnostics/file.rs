use super::StopReason;
use super::run::{CheckRuntime, Shared};
use crate::storage::SyncSink;
use std::{
    cell::RefCell,
    fs::{self, File, Metadata, OpenOptions},
    io::{self, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

pub(super) struct OwnedFile {
    directory: PathBuf,
    path: PathBuf,
    dir_created: bool,
    file_created: bool,
    dir_id: Option<(u64, u64)>,
    file_id: Option<(u64, u64)>,
    file: Option<File>,
}
fn identity(metadata: &Metadata) -> Option<(u64, u64)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some((metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        None
    }
}
fn verify(path: &Path, id: Option<(u64, u64)>, directory: bool) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    if id.is_none() || identity(&m) != id || m.file_type().is_symlink() || m.is_dir() != directory {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "check namespace identity changed",
        ));
    }
    Ok(())
}
impl OwnedFile {
    pub fn new(directory: &Path) -> Self {
        Self {
            directory: directory.to_path_buf(),
            path: directory.join("check.mcap"),
            dir_created: false,
            file_created: false,
            dir_id: None,
            file_id: None,
            file: None,
        }
    }
    pub fn parent(&self) -> &Path {
        self.directory.parent().unwrap_or_else(|| Path::new("/"))
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn create(&mut self) -> io::Result<()> {
        // Same exclusive trusted-parent contract as FileStore. A path metadata
        // check is not protection from an adversary replacing ancestor paths.
        if !fs::symlink_metadata(self.parent())?.file_type().is_dir() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        fs::create_dir(&self.directory)?;
        self.dir_created = true;
        self.dir_id = identity(&fs::symlink_metadata(&self.directory)?);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&self.path)?;
        self.file_created = true;
        self.file = Some(file);
        self.file_id = identity(
            &self
                .file
                .as_ref()
                .ok_or(io::ErrorKind::NotFound)?
                .metadata()?,
        );
        Ok(())
    }
    pub fn clone_file(&self) -> io::Result<File> {
        self.file
            .as_ref()
            .ok_or(io::ErrorKind::NotFound)?
            .try_clone()
    }
    pub fn settle(&mut self, length: &mut Option<u64>) -> io::Result<()> {
        let mut failure = None;
        if let Some(file) = &mut self.file {
            match file.metadata() {
                Ok(m) => *length = Some(m.len()),
                Err(e) => failure = Some(e),
            }
            // Even a metadata/flush error must not skip the real sync attempt.
            for result in [file.flush(), file.sync_all()] {
                if let Err(e) = result {
                    if failure.is_none() {
                        failure = Some(e);
                    }
                }
            }
            let namespace = verify(&self.directory, self.dir_id, true)
                .and_then(|()| verify(&self.path, self.file_id, false))
                .and_then(|()| File::open(&self.directory)?.sync_all());
            if let Err(e) = namespace {
                if failure.is_none() {
                    failure = Some(e);
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }
    pub fn close(&mut self) {
        drop(self.file.take());
    }
    pub fn cleanup(&mut self) -> io::Result<()> {
        if !self.dir_created {
            return Ok(());
        }
        verify(&self.directory, self.dir_id, true)?;
        if self.file_created {
            verify(&self.path, self.file_id, false)?;
            fs::remove_file(&self.path)?;
        }
        // Never recursively remove unknown files. DirectoryNotEmpty preserves
        // unexpected data and is returned as a separate cleanup failure.
        fs::remove_dir(&self.directory)?;
        File::open(self.parent())?.sync_all()
    }
}
/// Counts actual adapter writes, including seek rewrites. Reservation bounds are
/// conservative admission estimates; this is the physical-byte safety backstop.
pub(super) struct BoundedSink<'a, W, R> {
    inner: W,
    shared: Rc<RefCell<Shared<'a, R>>>,
    directory: PathBuf,
}
impl<'a, W, R> BoundedSink<'a, W, R> {
    pub fn new(inner: W, shared: Rc<RefCell<Shared<'a, R>>>, directory: &Path) -> Self {
        Self {
            inner,
            shared,
            directory: directory.to_path_buf(),
        }
    }
}
impl<W: Write, R: CheckRuntime> Write for BoundedSink<'_, W, R> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        let mut s = self.shared.borrow_mut();
        let remaining = s.limit.min(s.reserved).saturating_sub(s.bytes);
        let free = s.runtime.available_bytes(&self.directory)?;
        let space = free.saturating_sub(s.reserve);
        if bytes.len() as u64 > remaining || bytes.len() as u64 > space {
            s.blocked = Some(if bytes.len() as u64 > remaining {
                StopReason::WriteBudget
            } else {
                StopReason::DiskReserve
            });
            return Err(io::Error::new(
                io::ErrorKind::StorageFull,
                "check physical write limit",
            ));
        }
        let n = self.inner.write(bytes)?;
        s.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl<W: Seek, R> Seek for BoundedSink<'_, W, R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}
impl<W: SyncSink, R: CheckRuntime> SyncSink for BoundedSink<'_, W, R> {
    fn sync_all(&mut self) -> io::Result<()> {
        self.inner.sync_all()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    struct Runtime {
        free: u64,
    }
    impl CheckRuntime for Runtime {
        fn now_ns(&mut self) -> io::Result<u64> {
            Ok(0)
        }
        fn cancelled(&self) -> bool {
            false
        }
        fn available_bytes(&mut self, _: &Path) -> io::Result<u64> {
            Ok(self.free)
        }
        fn wait_until(&mut self, _: u64) -> io::Result<()> {
            Ok(())
        }
    }
    struct FaultSink {
        inner: io::Cursor<Vec<u8>>,
        writes: usize,
        fail_after: Option<usize>,
    }
    impl Write for FaultSink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.fail_after == Some(self.writes) {
                return Err(io::Error::from_raw_os_error(5));
            }
            self.writes += 1;
            self.inner.write(&bytes[..bytes.len().min(2)])
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Seek for FaultSink {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.inner.seek(pos)
        }
    }
    impl SyncSink for FaultSink {
        fn sync_all(&mut self) -> io::Result<()> {
            Err(io::Error::from_raw_os_error(5))
        }
    }
    fn state(r: &mut Runtime) -> Rc<RefCell<Shared<'_, Runtime>>> {
        Rc::new(RefCell::new(Shared {
            runtime: r,
            bytes: 0,
            reserved: 8,
            limit: 8,
            reserve: 1,
            last_ns: 0,
            blocked: None,
        }))
    }
    #[test]
    fn partial_writes_and_seek_rewrites_count_actual_bytes_without_exceeding_cap() {
        let mut r = Runtime { free: 1024 };
        let s = state(&mut r);
        let mut sink = BoundedSink::new(
            FaultSink {
                inner: io::Cursor::new(vec![]),
                writes: 0,
                fail_after: None,
            },
            Rc::clone(&s),
            Path::new("."),
        );
        sink.write_all(&[1; 5]).unwrap();
        assert_eq!(s.borrow().bytes, 5);
        sink.seek(SeekFrom::Start(0)).unwrap();
        sink.write_all(&[2; 2]).unwrap();
        assert_eq!(s.borrow().bytes, 7);
        assert_eq!(
            sink.write(&[3; 2]).unwrap_err().kind(),
            io::ErrorKind::StorageFull
        );
        assert_eq!(s.borrow().bytes, 7);
        assert_eq!(s.borrow().blocked, Some(StopReason::WriteBudget));
        assert_eq!(sink.inner.inner.get_ref().len(), 5);
    }
    #[test]
    fn partial_os_error_retains_prefix_count_and_original_error() {
        let mut r = Runtime { free: 1024 };
        let s = state(&mut r);
        let mut sink = BoundedSink::new(
            FaultSink {
                inner: io::Cursor::new(vec![]),
                writes: 0,
                fail_after: Some(1),
            },
            Rc::clone(&s),
            Path::new("."),
        );
        assert_eq!(sink.write_all(&[1; 5]).unwrap_err().raw_os_error(), Some(5));
        assert_eq!(s.borrow().bytes, 2);
        assert_eq!(s.borrow().blocked, None);
        assert_eq!(sink.sync_all().unwrap_err().raw_os_error(), Some(5));
    }
    #[test]
    fn reserve_backstop_prevents_underlying_write() {
        let mut r = Runtime { free: 1 };
        let s = state(&mut r);
        let mut sink = BoundedSink::new(
            FaultSink {
                inner: io::Cursor::new(vec![]),
                writes: 0,
                fail_after: None,
            },
            Rc::clone(&s),
            Path::new("."),
        );
        assert!(sink.write(&[1]).is_err());
        assert_eq!(sink.inner.writes, 0);
        assert_eq!(s.borrow().blocked, Some(StopReason::DiskReserve));
    }
}
