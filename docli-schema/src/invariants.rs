//! Hard invariant checks across DOCX package XML.

use std::collections::HashMap;

use crate::structural::{Severity, ValidationIssue};

/// Check hard invariants across the package XML.
///
/// Checks performed:
/// 1. No duplicate `w:id` attribute values across all XML parts.
/// 2. Relationship target paths referenced in `*/_rels/*.rels` files exist as
///    parts in the package (external URLs are skipped).
///
/// Returns `Vec<ValidationIssue>` containing only errors (no warnings).
pub fn check_invariants(package: &docli_core::Package) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    issues.extend(check_duplicate_w_ids(package));
    issues.extend(check_relationship_targets(package));

    issues
}

/// Collect all `w:id` attribute values across all XML parts.
/// Any value appearing more than once is a violation.
fn check_duplicate_w_ids(package: &docli_core::Package) -> Vec<ValidationIssue> {
    // Maps w:id value -> list of "part:line" locations where it appears.
    let mut id_locations: HashMap<String, Vec<String>> = HashMap::new();

    for (part_path, xml_bytes) in &package.xml_parts {
        let xml_str = match std::str::from_utf8(xml_bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let doc = match roxmltree::Document::parse(xml_str) {
            Ok(d) => d,
            Err(_) => continue,
        };

        for node in doc.descendants() {
            if !node.is_element() {
                continue;
            }
            // Match any attribute in any namespace with local name "id" that is
            // in the "w" namespace prefix. roxmltree resolves namespaces, so we
            // check via the namespace URI for WordprocessingML.
            for attr in node.attributes() {
                if attr.name() == "id" {
                    // Accept the w: namespace (WordprocessingML 2006) or plain "id" only
                    // when the attribute's namespace matches the w: URI.
                    let ns = attr.namespace().unwrap_or("");
                    if ns == "http://schemas.openxmlformats.org/wordprocessingml/2006/main" {
                        id_locations
                            .entry(attr.value().to_string())
                            .or_default()
                            .push(part_path.clone());
                    }
                }
            }
        }
    }

    let mut issues = Vec::new();
    for (id_value, locations) in &id_locations {
        if locations.len() > 1 {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                code: "DUPLICATE_W_ID",
                message: format!(
                    "Duplicate w:id value `{id_value}` found in parts: {}",
                    locations.join(", ")
                ),
            });
        }
    }

    issues
}

/// For each `*/_rels/*.rels` file, extract `Target` attributes from
/// `Relationship` elements. Non-external targets are resolved relative to the
/// rels file location and checked against `package.inventory`.
fn check_relationship_targets(package: &docli_core::Package) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for (part_path, xml_bytes) in &package.xml_parts {
        if !part_path.contains("/_rels/") && !part_path.starts_with("_rels/") {
            continue;
        }
        if !part_path.ends_with(".rels") {
            continue;
        }

        let xml_str = match std::str::from_utf8(xml_bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let doc = match roxmltree::Document::parse(xml_str) {
            Ok(d) => d,
            Err(_) => continue,
        };

        for node in doc.descendants() {
            if !node.is_element() || node.tag_name().name() != "Relationship" {
                continue;
            }
            let Some(target) = node.attribute("Target") else {
                continue;
            };

            // Skip external (http/https) and absolute ("/") targets.
            if target.starts_with("http://") || target.starts_with("https://") {
                continue;
            }

            // Resolve target relative to the directory containing the .rels file.
            // A rels file at `word/_rels/document.xml.rels` has a base dir of `word/`.
            let resolved = resolve_target(part_path, target);

            if !package.inventory.entries.contains_key(&resolved) {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    code: "MISSING_REL_TARGET",
                    message: format!(
                        "Relationship in `{part_path}` references missing part `{resolved}` (target: `{target}`)"
                    ),
                });
            }
        }
    }

    issues
}

/// Resolve a relationship `Target` value relative to a `.rels` file path.
///
/// A rels file at `word/_rels/document.xml.rels` has its "base" at `word/`.
/// So a target of `../media/image1.png` resolves to `word/../media/image1.png`
/// which canonicalises to `media/image1.png`.
///
/// A rels file at `_rels/.rels` (root) has its base at the ZIP root (`""`).
fn resolve_target(rels_path: &str, target: &str) -> String {
    // Absolute targets (start with '/') strip the leading slash.
    if let Some(stripped) = target.strip_prefix('/') {
        return stripped.to_string();
    }

    // Derive the base directory: strip `_rels/<name>.rels` to get the parent.
    // e.g. "word/_rels/document.xml.rels" → "word"
    //      "_rels/.rels"                  → ""
    let base = if let Some(idx) = rels_path.find("/_rels/") {
        &rels_path[..idx]
    } else {
        // rels file is at root: "_rels/.rels" → base is ""
        ""
    };

    // Join base with target and normalise ".." components.
    let joined = if base.is_empty() {
        target.to_string()
    } else {
        format!("{base}/{target}")
    };

    normalise_path(&joined)
}

/// Normalise a forward-slash path by resolving `..` components.
fn normalise_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            s => parts.push(s),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use docli_core::{Package, PartEntry, PartInventory};
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    use std::path::PathBuf;

    const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    fn make_package_with_xml(xml_parts: Vec<(&str, &str)>, extra_parts: &[&str]) -> Package {
        let mut entries = BTreeMap::new();
        let mut xml_map: HashMap<String, Vec<u8>> = HashMap::new();

        for (path, xml) in &xml_parts {
            let bytes = xml.as_bytes().to_vec();
            xml_map.insert(path.to_string(), bytes.clone());
            entries.insert(
                path.to_string(),
                PartEntry {
                    path: path.to_string(),
                    sha256: String::new(),
                    is_xml: true,
                    size_bytes: bytes.len() as u64,
                },
            );
        }

        for &path in extra_parts {
            entries.insert(
                path.to_string(),
                PartEntry {
                    path: path.to_string(),
                    sha256: String::new(),
                    is_xml: false,
                    size_bytes: 0,
                },
            );
        }

        Package {
            path: PathBuf::from("test.docx"),
            source_hash: String::new(),
            inventory: PartInventory { entries },
            xml_parts: xml_map,
            binary_parts: BTreeSet::new(),
        }
    }

    #[test]
    fn no_issues_when_no_duplicate_ids() {
        let xml = format!(
            r#"<?xml version="1.0"?>
            <w:document xmlns:w="{W_NS}">
              <w:body>
                <w:p w:id="1"/>
                <w:p w:id="2"/>
              </w:body>
            </w:document>"#
        );
        let pkg = make_package_with_xml(vec![("word/document.xml", &xml)], &[]);
        let issues = check_invariants(&pkg);
        assert!(
            issues.is_empty(),
            "Expected no issues for unique ids, got: {issues:?}"
        );
    }

    #[test]
    fn duplicate_w_id_across_parts_is_detected() {
        let xml_a = format!(
            r#"<?xml version="1.0"?>
            <w:document xmlns:w="{W_NS}">
              <w:body><w:p w:id="42"/></w:body>
            </w:document>"#
        );
        let xml_b = format!(
            r#"<?xml version="1.0"?>
            <w:document xmlns:w="{W_NS}">
              <w:body><w:p w:id="42"/></w:body>
            </w:document>"#
        );
        let pkg = make_package_with_xml(
            vec![
                ("word/document.xml", &xml_a),
                ("word/footer1.xml", &xml_b),
            ],
            &[],
        );
        let issues = check_invariants(&pkg);
        assert!(
            issues.iter().any(|i| i.code == "DUPLICATE_W_ID"),
            "Expected DUPLICATE_W_ID error, got: {issues:?}"
        );
    }

    #[test]
    fn missing_rel_target_is_detected() {
        let rels_xml = r#"<?xml version="1.0"?>
            <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
              <Relationship Id="rId1" Type="http://example.com/type" Target="media/image1.png"/>
            </Relationships>"#;
        // The part media/image1.png is NOT in the inventory.
        let pkg = make_package_with_xml(vec![("word/_rels/document.xml.rels", rels_xml)], &[]);
        let issues = check_invariants(&pkg);
        assert!(
            issues.iter().any(|i| i.code == "MISSING_REL_TARGET"),
            "Expected MISSING_REL_TARGET error, got: {issues:?}"
        );
    }

    #[test]
    fn existing_rel_target_produces_no_issue() {
        let rels_xml = r#"<?xml version="1.0"?>
            <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
              <Relationship Id="rId1" Type="http://example.com/type" Target="../media/image1.png"/>
            </Relationships>"#;
        // Target resolves from "word" base to "media/image1.png"
        let pkg = make_package_with_xml(
            vec![("word/_rels/document.xml.rels", rels_xml)],
            &["media/image1.png"],
        );
        let issues = check_invariants(&pkg);
        assert!(
            !issues.iter().any(|i| i.code == "MISSING_REL_TARGET"),
            "Expected no MISSING_REL_TARGET for existing part, got: {issues:?}"
        );
    }

    #[test]
    fn resolve_target_handles_dotdot() {
        assert_eq!(
            resolve_target("word/_rels/document.xml.rels", "../media/image1.png"),
            "media/image1.png"
        );
    }

    #[test]
    fn resolve_target_handles_root_rels() {
        assert_eq!(
            resolve_target("_rels/.rels", "word/document.xml"),
            "word/document.xml"
        );
    }
}
