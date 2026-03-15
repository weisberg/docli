use std::path::{Path, PathBuf};

use docli_core::DocliError;

pub struct KbResolver {
    kb_root: PathBuf,
}

impl KbResolver {
    /// Create a resolver with an explicit root.
    pub fn new(kb_root: impl Into<PathBuf>) -> Self {
        Self {
            kb_root: kb_root.into(),
        }
    }

    /// Create from environment (DOCLI_KB_ROOT env var, or ~/.config/docli/kb/ default).
    pub fn from_env() -> Self {
        let kb_root = if let Ok(val) = std::env::var("DOCLI_KB_ROOT") {
            PathBuf::from(val)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
            PathBuf::from(home).join(".config").join("docli").join("kb")
        };
        Self { kb_root }
    }

    /// Resolve a kb:// URI to a filesystem path.
    /// kb://category/name.yaml → {kb_root}/templates/docli/category/name.yaml
    /// Returns Err(RefNotFound) if the file doesn't exist.
    pub fn resolve(&self, uri: &str) -> Result<PathBuf, DocliError> {
        let rest = uri
            .strip_prefix("kb://")
            .ok_or_else(|| DocliError::RefNotFound {
                reference: uri.to_string(),
            })?;

        let path = self.kb_root.join("templates").join("docli").join(rest);

        if path.exists() {
            Ok(path)
        } else {
            Err(DocliError::RefNotFound {
                reference: uri.to_string(),
            })
        }
    }

    /// List all entries in a category directory.
    /// Returns file stems (without extension) alphabetically sorted.
    pub fn list_entries(&self, category: &str) -> Result<Vec<String>, DocliError> {
        let dir = self
            .kb_root
            .join("templates")
            .join("docli")
            .join(category);

        if !dir.exists() {
            return Err(DocliError::RefNotFound {
                reference: format!("kb://{}/", category),
            });
        }

        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| DocliError::InvalidSpec {
            message: e.to_string(),
        })? {
            let entry = entry.map_err(|e| DocliError::InvalidSpec {
                message: e.to_string(),
            })?;
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    entries.push(stem.to_string());
                }
            }
        }
        entries.sort();
        Ok(entries)
    }

    /// Validate all .yaml files in the KB can be parsed.
    /// Returns list of (uri, error) pairs for any that fail.
    pub fn validate_all(&self) -> Vec<(String, String)> {
        let templates_root = self.kb_root.join("templates").join("docli");
        let mut failures = Vec::new();
        self.validate_dir(&templates_root, &templates_root, &mut failures);
        failures
    }

    fn validate_dir(&self, root: &Path, dir: &Path, failures: &mut Vec<(String, String)>) {
        let read = match std::fs::read_dir(dir) {
            Ok(r) => r,
            Err(_) => return,
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.validate_dir(root, &path, failures);
            } else if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                let uri = format!("kb://{}", rel.display());
                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        failures.push((uri, e.to_string()));
                        continue;
                    }
                };
                if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                    failures.push((uri, e.to_string()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_kb(dir: &std::path::Path) {
        let templates = dir.join("templates").join("docli").join("cat");
        fs::create_dir_all(&templates).unwrap();
        fs::write(templates.join("alpha.yaml"), "key: value\n").unwrap();
        fs::write(templates.join("beta.yaml"), "key: value2\n").unwrap();
        fs::write(templates.join("gamma.yaml"), "key: value3\n").unwrap();
    }

    #[test]
    fn from_env_uses_default_path() {
        // Remove env override so we get the default.
        std::env::remove_var("DOCLI_KB_ROOT");
        let resolver = KbResolver::from_env();
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
        let expected = PathBuf::from(home)
            .join(".config")
            .join("docli")
            .join("kb");
        assert_eq!(resolver.kb_root, expected);
    }

    #[test]
    fn list_entries_returns_sorted_names() {
        let tmp = TempDir::new().unwrap();
        make_kb(tmp.path());
        let resolver = KbResolver::new(tmp.path());
        let entries = resolver.list_entries("cat").unwrap();
        assert_eq!(entries, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn resolve_returns_path_when_file_exists() {
        let tmp = TempDir::new().unwrap();
        make_kb(tmp.path());
        let resolver = KbResolver::new(tmp.path());
        let path = resolver.resolve("kb://cat/alpha.yaml").unwrap();
        assert!(path.exists());
    }

    #[test]
    fn resolve_returns_ref_not_found_when_missing() {
        let tmp = TempDir::new().unwrap();
        make_kb(tmp.path());
        let resolver = KbResolver::new(tmp.path());
        let err = resolver.resolve("kb://cat/nonexistent.yaml").unwrap_err();
        assert!(matches!(err, DocliError::RefNotFound { .. }));
    }
}
