use std::collections::HashMap;

use docli_core::DocliError;
use minijinja::Environment;

/// Render a template string with minijinja, then return the rendered string.
/// Automatically injects `date` = today's date (YYYY-MM-DD) if not in vars.
pub fn render_template(
    content: &str,
    vars: &HashMap<String, serde_json::Value>,
) -> Result<String, DocliError> {
    let mut env = Environment::new();
    env.add_template("t", content)
        .map_err(|e| DocliError::InvalidSpec {
            message: e.to_string(),
        })?;
    let tmpl = env.get_template("t").unwrap();

    let mut ctx = vars.clone();
    ctx.entry("date".to_string())
        .or_insert_with(|| serde_json::Value::String(today_date()));

    tmpl.render(&ctx).map_err(|e| DocliError::InvalidSpec {
        message: e.to_string(),
    })
}

fn today_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let days_since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86400;

    let z = days_since_epoch as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_provided_vars() {
        let mut vars = HashMap::new();
        vars.insert(
            "name".to_string(),
            serde_json::Value::String("Alice".to_string()),
        );
        let result = render_template("Hello, {{ name }}!", &vars).unwrap();
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn injects_date_when_not_provided() {
        let vars = HashMap::new();
        let result = render_template("Today is {{ date }}.", &vars).unwrap();
        // Should contain a date-like string YYYY-MM-DD
        assert!(result.contains("Today is "));
        // Validate format: "Today is YYYY-MM-DD."
        let date_part = result
            .strip_prefix("Today is ")
            .unwrap()
            .strip_suffix('.')
            .unwrap();
        assert_eq!(date_part.len(), 10);
        assert_eq!(date_part.chars().nth(4), Some('-'));
        assert_eq!(date_part.chars().nth(7), Some('-'));
    }

    #[test]
    fn caller_supplied_date_is_not_overridden() {
        let mut vars = HashMap::new();
        vars.insert(
            "date".to_string(),
            serde_json::Value::String("1999-01-01".to_string()),
        );
        let result = render_template("{{ date }}", &vars).unwrap();
        assert_eq!(result, "1999-01-01");
    }

    #[test]
    fn unknown_variable_renders_empty_by_default() {
        let vars = HashMap::new();
        // minijinja renders undefined as empty string by default (undefined behavior)
        let result = render_template("{{ unknown_var }}", &vars).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn invalid_template_syntax_returns_error() {
        let vars = HashMap::new();
        let err = render_template("{% for %}", &vars).unwrap_err();
        assert!(matches!(err, DocliError::InvalidSpec { .. }));
    }

    #[test]
    fn today_date_returns_valid_format() {
        let date = today_date();
        assert_eq!(date.len(), 10);
        assert_eq!(date.chars().nth(4), Some('-'));
        assert_eq!(date.chars().nth(7), Some('-'));
        // Should start with "20" for dates in this century
        assert!(date.starts_with("20"), "date was: {date}");
    }
}
