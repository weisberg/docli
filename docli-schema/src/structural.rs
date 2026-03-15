//! Structural validity checks for DOCX packages.

/// Severity level of a validation issue.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// A single validation finding from structural or invariant checking.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
}

/// Check structural validity of a DOCX package.
///
/// Required parts checked:
/// - `[Content_Types].xml` (always in ZIP root)
/// - `word/document.xml`
/// - `word/_rels/document.xml.rels`
///
/// Warns if `_rels/.rels` is absent.
pub fn check_structural(package: &docli_core::Package) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    let required_errors: &[(&str, &str)] = &[
        ("[Content_Types].xml", "MISSING_CONTENT_TYPES"),
        ("word/document.xml", "MISSING_DOCUMENT_XML"),
        ("word/_rels/document.xml.rels", "MISSING_DOCUMENT_RELS"),
    ];

    for (part, code) in required_errors {
        if !package.inventory.entries.contains_key(*part) {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                code,
                message: format!("Required DOCX part is missing: {part}"),
            });
        }
    }

    if !package.inventory.entries.contains_key("_rels/.rels") {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            code: "MISSING_ROOT_RELS",
            message: "Root relationship part `_rels/.rels` is absent".to_string(),
        });
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use docli_core::{Package, PartEntry, PartInventory};
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    use std::path::PathBuf;

    fn make_package(parts: &[&str]) -> Package {
        let mut entries = BTreeMap::new();
        for &p in parts {
            entries.insert(
                p.to_string(),
                PartEntry {
                    path: p.to_string(),
                    sha256: String::new(),
                    is_xml: true,
                    size_bytes: 0,
                },
            );
        }
        Package {
            path: PathBuf::from("test.docx"),
            source_hash: String::new(),
            inventory: PartInventory { entries },
            xml_parts: HashMap::new(),
            binary_parts: BTreeSet::new(),
        }
    }

    #[test]
    fn valid_package_produces_no_errors() {
        let pkg = make_package(&[
            "[Content_Types].xml",
            "word/document.xml",
            "word/_rels/document.xml.rels",
            "_rels/.rels",
        ]);
        let issues = check_structural(&pkg);
        assert!(issues.is_empty(), "Expected no issues, got: {issues:?}");
    }

    #[test]
    fn missing_document_xml_is_an_error() {
        let pkg = make_package(&[
            "[Content_Types].xml",
            "word/_rels/document.xml.rels",
            "_rels/.rels",
        ]);
        let issues = check_structural(&pkg);
        assert!(
            issues
                .iter()
                .any(|i| i.code == "MISSING_DOCUMENT_XML" && i.severity == Severity::Error),
            "Expected MISSING_DOCUMENT_XML error"
        );
    }

    #[test]
    fn missing_root_rels_is_a_warning() {
        let pkg = make_package(&[
            "[Content_Types].xml",
            "word/document.xml",
            "word/_rels/document.xml.rels",
        ]);
        let issues = check_structural(&pkg);
        assert!(
            issues
                .iter()
                .any(|i| i.code == "MISSING_ROOT_RELS" && i.severity == Severity::Warning),
            "Expected MISSING_ROOT_RELS warning"
        );
        // Should be no errors (only the warning).
        assert!(
            !issues.iter().any(|i| i.severity == Severity::Error),
            "Expected no errors when only _rels/.rels is missing"
        );
    }

    #[test]
    fn missing_content_types_is_an_error() {
        let pkg = make_package(&[
            "word/document.xml",
            "word/_rels/document.xml.rels",
            "_rels/.rels",
        ]);
        let issues = check_structural(&pkg);
        assert!(
            issues
                .iter()
                .any(|i| i.code == "MISSING_CONTENT_TYPES" && i.severity == Severity::Error),
            "Expected MISSING_CONTENT_TYPES error"
        );
    }
}
