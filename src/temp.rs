use std::env;

use backon::BlockingRetryable;
use backon::ExponentialBuilder;
use lazy_static::lazy_static;
use uuid::Uuid;

fn get_exec_name() -> Option<String> {
    Some(env::current_exe().ok()?.file_name()?.to_str()?.to_string())
}

lazy_static! {
    static ref EXEC_NAME: String = get_exec_name().unwrap_or("rust".to_string());
}

fn get_temp_name(extension: Option<&str>) -> String {
    let id = Uuid::new_v4();
    format!(
        "{}_{}{}",
        *EXEC_NAME,
        id.to_string(),
        extension.unwrap_or("")
    )
}

fn get_temp_path(extension: Option<&str>) -> std::path::PathBuf {
    let mut path = env::temp_dir();
    path.push(get_temp_name(extension));
    path
}

pub struct TempFile {
    path: std::path::PathBuf,
}

pub struct TempDir {
    path: std::path::PathBuf,
}

impl TempFile {
    pub fn new(extension: Option<&str>) -> std::io::Result<Self> {
        let result = Self {
            path: get_temp_path(extension),
        };
        std::fs::File::create(&result.path)?;
        log::trace!("Created temp file: {:?}", result.path);
        Ok(result)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        log::trace!("Removing temp file: {:?}", self.path);
        remove(std::mem::take(&mut self.path));
    }
}

impl TempDir {
    pub fn new() -> std::io::Result<Self> {
        let result = Self {
            path: get_temp_path(None),
        };
        std::fs::create_dir(&result.path)?;
        log::trace!("Created temp dir: {:?}", result.path);
        Ok(result)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        log::trace!("Removing temp dir: {:?}", self.path);
        remove(std::mem::take(&mut self.path));
    }
}

fn remove(path: std::path::PathBuf) {
    let is_dir = path.is_dir();
    let remove = move || {
        if is_dir {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        }
    };
    if let Err(_) = remove() {
        rayon::spawn(move || {
            if let Err(e) = remove.retry(&ExponentialBuilder::default()).call() {
                log::error!(
                    "Failed to remove temp {}: {:?}",
                    if is_dir { "dir" } else { "file" },
                    e
                );
            }
        });
    }
}

pub trait FileDetails {
    fn get_path(&self) -> &std::path::Path;
    fn get_name(&self) -> Option<&str> {
        Some(self.get_path().file_name()?.to_str()?)
    }
}

impl FileDetails for TempFile {
    fn get_path(&self) -> &std::path::Path {
        &self.path
    }
}

impl FileDetails for TempDir {
    fn get_path(&self) -> &std::path::Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_delete_temp_file() {
        let temp_file = TempFile::new(Some(".txt")).unwrap();
        let path = temp_file.path.clone();
        assert!(path.exists());
        drop(temp_file);
        assert!(!path.exists());
    }

    #[test]
    fn should_delete_temp_dir() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path.clone();
        assert!(path.exists());

        let mut file_path = path.clone();
        file_path.push("test.txt");
        std::fs::File::create(&file_path).unwrap();
        assert!(file_path.exists());

        drop(temp_dir);
        assert!(!path.exists());
        assert!(!file_path.exists());
    }
}
