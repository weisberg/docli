//! Structural validation and repair for DOCX packages.

pub mod invariants;
pub mod redline;
pub mod repair;
pub mod structural;

pub use structural::{Severity, ValidationIssue};
