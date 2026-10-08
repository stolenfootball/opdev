//! Cooperating writer primitives shared by workflow journals and local state.
use anyhow::{Context, Result, ensure};
use std::{
    fs::{self, File},
    path::Path,
};

struct WriterLock {
    file: File,
    released: bool,
}

impl WriterLock {
    fn acquire(path: &Path) -> Result<Self> {
        if let Ok(meta) = fs::symlink_metadata(path) {
            ensure!(
                meta.is_file() && !meta.file_type().is_symlink(),
                "Writer lock must be a regular file"
            );
        }
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        file.try_lock().context(
            "Another workflow writer is active; inspect the latest state before retrying",
        )?;
        Ok(Self {
            file,
            released: false,
        })
    }

    fn release(&mut self) -> std::io::Result<()> {
        self.file.unlock()?;
        self.released = true;
        Ok(())
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        if !self.released {
            // Covers unwinding, not process death. Normal-path failures are reported.
            let _ = self.file.unlock();
        }
    }
}

pub(super) fn with_lock<T>(path: &Path, operation: impl FnOnce() -> Result<T>) -> Result<T> {
    let mut lock = WriterLock::acquire(path)?;
    let result = operation();
    let released = lock.release();
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error).context("Could not release the writer lock; the write may already be committed. Inspect current state before another attempt"),
        (Err(error), Err(unlock)) => Err(error).context(format!("Writer failed and lock release also failed ({unlock}); inspect current state before another attempt")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_release_does_not_wait_for_a_duplicate_handle() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("writer.lock");
        let mut owner = WriterLock::acquire(&path)?;
        let duplicate = owner.file.try_clone()?;
        assert!(WriterLock::acquire(&path).is_err());
        owner.release()?;
        let mut next = WriterLock::acquire(&path)?;
        next.release()?;
        drop(duplicate);
        Ok(())
    }

    #[test]
    fn error_exit_and_guard_drop_release_without_hidden_acquisition_retries() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("writer.lock");
        let failed: Result<()> = with_lock(&path, || anyhow::bail!("invalid expected head"));
        assert!(failed.is_err());
        assert!(with_lock(&path, || Ok(true))?);
        let owner = WriterLock::acquire(&path)?;
        let duplicate = owner.file.try_clone()?;
        drop(owner);
        assert!(with_lock(&path, || Ok(true))?);
        drop(duplicate);
        Ok(())
    }
}
