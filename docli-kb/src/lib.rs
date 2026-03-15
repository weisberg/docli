//! Knowledge-base resolution, rule loading, and template rendering.

pub mod resolver;
pub mod rules;
pub mod template;

pub use resolver::KbResolver;
pub use rules::{load_all_rules, load_rule, Rule};
pub use template::render_template;
