use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
    persist: bool,
}

impl TempDir {
    pub fn new(prefix: &str) -> Result<Self, String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir_name = format!("smartcut_{}_{}_{}", prefix, timestamp, id);
        let path = std::env::temp_dir().join(dir_name);

        std::fs::create_dir_all(&path)
            .map_err(|e| format!("No se pudo crear directorio temporal {:?}: {}", path, e))?;

        Ok(Self {
            path,
            persist: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file_path(&self, filename: &str) -> PathBuf {
        self.path.join(filename)
    }

    #[allow(dead_code)]
    pub fn keep(&mut self) {
        self.persist = true;
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if !self.persist && self.path.exists() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
