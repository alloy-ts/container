use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

#[napi]
pub struct BuildFile;

#[napi]
impl BuildFile {
  #[napi]
  pub fn resolve_path(context_dir: String) -> Result<Option<String>> {
    let context_path = Path::new(&context_dir);
    let dockerfile = context_path.join("Dockerfile");
    let containerfile = context_path.join("Containerfile");

    if dockerfile.exists() {
      Ok(dockerfile.to_str().map(|s| s.to_string()))
    } else if containerfile.exists() {
      Ok(containerfile.to_str().map(|s| s.to_string()))
    } else {
      Ok(None)
    }
  }
}

#[napi]
pub struct BufferedCopyReader {
  file_path: PathBuf,
  chunk_size: usize,
  file: File,
  finished: bool,
}

#[napi]
impl BufferedCopyReader {
  #[napi(constructor)]
  pub fn new(file_path: String, chunk_size: Option<u32>) -> Result<Self> {
    let size = chunk_size.unwrap_or(4 * 1024 * 1024) as usize;
    let path = PathBuf::from(&file_path);
    let file = File::open(&path).map_err(|e| Error::from_reason(format!("Failed to open file '{}': {}", file_path, e)))?;
    Ok(Self {
      file_path: path,
      chunk_size: size,
      file,
      finished: false,
    })
  }

  #[napi]
  pub fn next_chunk(&mut self) -> Result<Option<Buffer>> {
    if self.finished {
      return Ok(None);
    }

    let mut buffer = vec![0u8; self.chunk_size];
    match self.file.read(&mut buffer) {
      Ok(0) => {
        self.finished = true;
        Ok(None)
      }
      Ok(n) => {
        if n < self.chunk_size {
          self.finished = true;
        }
        buffer.truncate(n);
        Ok(Some(Buffer::from(buffer)))
      }
      Err(e) => Err(Error::from_reason(e.to_string())),
    }
  }

  #[napi(getter)]
  pub fn has_finished(&self) -> bool {
    self.finished
  }

  #[napi]
  pub fn reset(&mut self) -> Result<()> {
    let file = File::open(&self.file_path)
      .map_err(|e| Error::from_reason(format!("Failed to reopen file '{}': {}", self.file_path.display(), e)))?;
    self.file = file;
    self.finished = false;
    Ok(())
  }
}

#[napi]
pub struct BuildFsSync {
  context_dir: PathBuf,
}

impl BuildFsSync {
  fn safe_resolve_path(&self, relative_or_abs: &str) -> Result<PathBuf> {
    let rel_path = Path::new(relative_or_abs);

    let clean_rel: PathBuf = rel_path
      .components()
      .filter(|c| matches!(c, Component::Normal(_)))
      .collect();

    let full_path = self.context_dir.join(clean_rel);

    let context_canonical = self
      .context_dir
      .canonicalize()
      .unwrap_or_else(|_| self.context_dir.clone());

    if full_path.exists() {
      if let Ok(canonical) = full_path.canonicalize() {
        if !canonical.starts_with(&context_canonical) {
          return Err(Error::from_reason(format!(
            "Path '{}' escapes context directory",
            relative_or_abs
          )));
        }
        return Ok(canonical);
      }
    }

    Ok(full_path)
  }
}

#[napi]
impl BuildFsSync {
  #[napi(constructor)]
  pub fn new(context_dir: String) -> Result<Self> {
    let path = PathBuf::from(&context_dir);
    Ok(Self { context_dir: path })
  }

  #[napi]
  pub fn accept_stage(&self, stage: String) -> bool {
    stage == "fssync"
  }

  #[napi]
  pub fn handle_walk(&self, follow_paths: Vec<String>) -> Result<Vec<String>> {
    let mut entries = Vec::new();
    for p in follow_paths {
      if let Ok(target) = self.safe_resolve_path(&p) {
        if target.exists() {
          entries.push(target.to_string_lossy().to_string());
        }
      }
    }
    Ok(entries)
  }

  #[napi]
  pub fn read_file_offset(&self, relative_path: String, offset: i64, size: i64) -> Result<Buffer> {
    if offset < 0 || size < 0 {
      return Err(Error::from_reason("Offset and size must be non-negative"));
    }
    let full_path = self.safe_resolve_path(&relative_path)?;
    let mut file = File::open(&full_path)
      .map_err(|e| Error::from_reason(format!("Failed to open '{}': {}", full_path.display(), e)))?;
    file
      .seek(SeekFrom::Start(offset as u64))
      .map_err(|e| Error::from_reason(e.to_string()))?;

    let mut buf = vec![0u8; size as usize];
    let n = file
      .read(&mut buf)
      .map_err(|e| Error::from_reason(e.to_string()))?;
    buf.truncate(n);
    Ok(Buffer::from(buf))
  }
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct TerminalCommandInfo {
  pub command_type: String,
  pub code: String,
  pub rows: u16,
  pub cols: u16,
}

#[napi]
pub struct TerminalCommand;

#[napi]
impl TerminalCommand {
  #[napi]
  pub fn create_winch(rows: u16, cols: u16) -> TerminalCommandInfo {
    TerminalCommandInfo {
      command_type: "terminal".to_string(),
      code: "winch".to_string(),
      rows,
      cols,
    }
  }

  #[napi]
  pub fn create_ack() -> TerminalCommandInfo {
    TerminalCommandInfo {
      command_type: "terminal".to_string(),
      code: "ack".to_string(),
      rows: 0,
      cols: 0,
    }
  }
}
