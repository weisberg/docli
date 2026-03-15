use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde::Serialize;

#[derive(Debug, Args)]
pub struct DoctorArgs {}

#[derive(Debug, Serialize)]
pub struct DoctorData {
    pub adapters: Vec<AdapterCheck>,
    pub kb_path_exists: bool,
    pub kb_writeable: bool,
    pub same_filesystem: bool,
}

#[derive(Debug, Serialize)]
pub struct AdapterCheck {
    pub name: String,
    pub available: bool,
}

pub fn run(args: &DoctorArgs, kb_path: &PathBuf) -> Result<DoctorData, docli_core::DocliError> {
    let _ = args;
    let adapters = ["pandoc", "soffice", "pdftoppm"]
        .into_iter()
        .map(|name| AdapterCheck {
            name: name.to_string(),
            available: executable_in_path(name),
        })
        .collect::<Vec<_>>();

    let kb_path_exists = kb_path.exists();
    let kb_writeable = check_dir_write(kb_path)?;
    let same_filesystem = same_filesystem(kb_path, std::env::current_dir()?.as_path());

    Ok(DoctorData {
        adapters,
        kb_path_exists,
        kb_writeable,
        same_filesystem,
    })
}

fn executable_in_path(name: &str) -> bool {
    if Path::new(name).exists() {
        return true;
    }
    for directory in std::env::var_os("PATH")
        .unwrap_or_default()
        .to_string_lossy()
        .split(':')
    {
        if Path::new(directory).join(name).exists() {
            return true;
        }
        #[cfg(windows)]
        if Path::new(directory).join(format!("{name}.exe")).exists() {
            return true;
        }
    }
    false
}

fn check_dir_write(path: &Path) -> Result<bool, docli_core::DocliError> {
    if path.exists() && !path.is_dir() {
        return Ok(false);
    }
    let temp_path = path.join(".docli-doctor-writecheck");
    fs::create_dir_all(path).map_err(|error| docli_core::DocliError::CommitFailed {
        message: error.to_string(),
    })?;
    match fs::write(&temp_path, "ok") {
        Ok(_) => {
            let _ = fs::remove_file(&temp_path);
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Ok(false),
        Err(error) => Err(docli_core::DocliError::CommitFailed {
            message: error.to_string(),
        }),
    }
}

fn same_filesystem(left: &Path, right: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let left = left.metadata().ok().and_then(|meta| Some(meta.dev()));
        let right = right.metadata().ok().and_then(|meta| Some(meta.dev()));
        match (left, right) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        // Conservative fallback for non-Unix platforms.
        let left = left.exists();
        let right = right.exists();
        left && right
    }
}
