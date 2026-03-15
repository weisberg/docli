use std::path::Path;

use docli_core::DocliError;
use serde::{Deserialize, Serialize};

/// A loaded rule file with YAML frontmatter + markdown body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub title: String,
    pub category: String,
    pub body: String,
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
struct Frontmatter {
    id: String,
    title: String,
    category: String,
    #[serde(default)]
    tags: Vec<String>,
}

/// Load a rule from a file (markdown with YAML frontmatter `---\n...\n---\n`).
pub fn load_rule(path: &Path) -> Result<Rule, DocliError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            DocliError::RefNotFound {
                reference: path.display().to_string(),
            }
        } else {
            DocliError::InvalidSpec {
                message: e.to_string(),
            }
        }
    })?;

    parse_rule(&content)
}

fn parse_rule(content: &str) -> Result<Rule, DocliError> {
    let after_open = content
        .strip_prefix("---\n")
        .ok_or_else(|| DocliError::InvalidSpec {
            message: "Rule file must start with '---'".to_string(),
        })?;

    let close_pos = after_open
        .find("\n---\n")
        .ok_or_else(|| DocliError::InvalidSpec {
            message: "Rule file frontmatter is not properly closed with '---'".to_string(),
        })?;

    let frontmatter_str = &after_open[..close_pos];
    let body = after_open[close_pos + 5..].to_string(); // skip "\n---\n"

    let fm: Frontmatter =
        serde_yaml::from_str(frontmatter_str).map_err(|e| DocliError::InvalidSpec {
            message: e.to_string(),
        })?;

    Ok(Rule {
        id: fm.id,
        title: fm.title,
        category: fm.category,
        tags: fm.tags,
        body,
    })
}

/// Load all rules from {kb_root}/templates/docli/rules/*.md
pub fn load_all_rules(kb_root: &Path) -> Result<Vec<Rule>, DocliError> {
    let rules_dir = kb_root.join("templates").join("docli").join("rules");

    if !rules_dir.exists() {
        return Ok(Vec::new());
    }

    let mut rules = Vec::new();
    for entry in std::fs::read_dir(&rules_dir).map_err(|e| DocliError::InvalidSpec {
        message: e.to_string(),
    })? {
        let entry = entry.map_err(|e| DocliError::InvalidSpec {
            message: e.to_string(),
        })?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            rules.push(load_rule(&path)?);
        }
    }

    rules.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const SAMPLE_RULE: &str = "\
---
id: test-001
title: Test Rule
category: testing
tags:
  - alpha
  - beta
---
# Test Rule

This is the body.
";

    #[test]
    fn load_rule_parses_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("rule.md");
        fs::write(&path, SAMPLE_RULE).unwrap();

        let rule = load_rule(&path).unwrap();
        assert_eq!(rule.id, "test-001");
        assert_eq!(rule.title, "Test Rule");
        assert_eq!(rule.category, "testing");
        assert_eq!(rule.tags, vec!["alpha", "beta"]);
        assert!(rule.body.contains("This is the body."));
    }

    #[test]
    fn missing_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.md");
        let err = load_rule(&path).unwrap_err();
        assert!(matches!(err, DocliError::RefNotFound { .. }));
    }

    #[test]
    fn load_all_rules_loads_multiple_files() {
        let tmp = TempDir::new().unwrap();
        let rules_dir = tmp.path().join("templates").join("docli").join("rules");
        fs::create_dir_all(&rules_dir).unwrap();

        let rule_a = "---\nid: rule-a\ntitle: Rule A\ncategory: cat\n---\nBody A\n";
        let rule_b = "---\nid: rule-b\ntitle: Rule B\ncategory: cat\n---\nBody B\n";
        fs::write(rules_dir.join("a.md"), rule_a).unwrap();
        fs::write(rules_dir.join("b.md"), rule_b).unwrap();

        let rules = load_all_rules(tmp.path()).unwrap();
        assert_eq!(rules.len(), 2);
        // sorted by id
        assert_eq!(rules[0].id, "rule-a");
        assert_eq!(rules[1].id, "rule-b");
    }

    #[test]
    fn missing_frontmatter_delimiter_returns_error() {
        let content = "id: test\ntitle: No Delimiters\n";
        let err = parse_rule(content).unwrap_err();
        assert!(matches!(err, DocliError::InvalidSpec { .. }));
    }
}
