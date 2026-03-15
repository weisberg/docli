//! Auto-repair utilities for DOCX XML content.

use quick_xml::{
    events::{BytesStart, Event},
    Reader, Writer,
};

use docli_core::DocliError;

/// Ensures all `<w:t>` elements with leading/trailing whitespace have
/// `xml:space="preserve"` attribute. Operates on raw XML bytes.
///
/// Returns modified XML bytes (a clone is always returned; callers may compare
/// with the input to detect whether any changes were made).
pub fn ensure_xml_space_preserve(xml: &[u8]) -> Result<Vec<u8>, DocliError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);

    let mut writer = Writer::new(Vec::new());

    // We process events in a small state machine:
    //   - When we see a `<w:t>` start tag, buffer it.
    //   - Collect the immediately following text event.
    //   - Emit the (possibly modified) start tag, then the text.
    //   - `</w:t>` and all other events are passed through directly.

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"w:t" => {
                // Peek at the next event to get the text content.
                let start_tag = e.to_owned();
                buf.clear();

                // Read the text content (may be Text or empty).
                let text_event = reader.read_event_into(&mut buf).map_err(|e| {
                    DocliError::InvalidDocx {
                        message: e.to_string(),
                    }
                })?;

                match text_event {
                    Event::Text(ref t) => {
                        let text = t.unescape().map_err(|e| DocliError::InvalidDocx {
                            message: e.to_string(),
                        })?;

                        let needs_preserve = text.starts_with(' ')
                            || text.ends_with(' ')
                            || text.starts_with('\t')
                            || text.ends_with('\t');

                        let has_preserve = start_tag
                            .attributes()
                            .flatten()
                            .any(|a| a.key.as_ref() == b"xml:space");

                        let final_start = if needs_preserve && !has_preserve {
                            add_xml_space_preserve(&start_tag)?
                        } else {
                            start_tag
                        };

                        writer
                            .write_event(Event::Start(final_start))
                            .map_err(|e| DocliError::InvalidDocx {
                                message: e.to_string(),
                            })?;
                        writer
                            .write_event(Event::Text(t.to_owned()))
                            .map_err(|e| DocliError::InvalidDocx {
                                message: e.to_string(),
                            })?;
                    }
                    other => {
                        // No text content immediately after <w:t> — just emit both.
                        writer
                            .write_event(Event::Start(start_tag))
                            .map_err(|e| DocliError::InvalidDocx {
                                message: e.to_string(),
                            })?;
                        writer
                            .write_event(other)
                            .map_err(|e| DocliError::InvalidDocx {
                                message: e.to_string(),
                            })?;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(event) => {
                writer
                    .write_event(event)
                    .map_err(|e| DocliError::InvalidDocx {
                        message: e.to_string(),
                    })?;
            }
            Err(e) => {
                return Err(DocliError::InvalidDocx {
                    message: e.to_string(),
                });
            }
        }
        buf.clear();
    }

    Ok(writer.into_inner())
}

/// Clone `start` and append an `xml:space="preserve"` attribute.
fn add_xml_space_preserve(start: &BytesStart<'_>) -> Result<BytesStart<'static>, DocliError> {
    let mut new_start = BytesStart::new(
        String::from_utf8(start.name().as_ref().to_vec()).map_err(|e| {
            DocliError::InvalidDocx {
                message: e.to_string(),
            }
        })?,
    );
    // Copy existing attributes.
    for attr in start.attributes().flatten() {
        new_start.push_attribute(attr);
    }
    new_start.push_attribute(("xml:space", "preserve"));
    Ok(new_start)
}

/// Repairs `w:id` overflow: renumbers any `w:id` values >= `0x7FFFFFFF`
/// to valid sequential values starting from 1 (avoiding collisions with
/// existing valid ids already in the document).
pub fn repair_durable_id_overflow(xml: &[u8]) -> Result<Vec<u8>, DocliError> {
    // Two-pass approach:
    //   Pass 1: collect all existing valid w:id values (< 0x7FFFFFFF).
    //   Pass 2: stream-transform, renumbering overflowing values.

    let existing_ids = collect_valid_w_ids(xml)?;

    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);

    let mut writer = Writer::new(Vec::new());
    let mut buf = Vec::new();

    // Next candidate sequential id (skip ones already in use).
    let mut next_id: u64 = 1;
    let mut get_next = |used: &std::collections::HashSet<u64>| -> u64 {
        while used.contains(&next_id) {
            next_id += 1;
        }
        let id = next_id;
        next_id += 1;
        id
    };

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let rewritten = rewrite_overflow_ids(e, &existing_ids, &mut get_next)?;
                writer
                    .write_event(Event::Start(rewritten))
                    .map_err(|e| DocliError::InvalidDocx {
                        message: e.to_string(),
                    })?;
            }
            Ok(Event::Empty(ref e)) => {
                let rewritten = rewrite_overflow_ids(e, &existing_ids, &mut get_next)?;
                writer
                    .write_event(Event::Empty(rewritten))
                    .map_err(|e| DocliError::InvalidDocx {
                        message: e.to_string(),
                    })?;
            }
            Ok(Event::Eof) => break,
            Ok(event) => {
                writer
                    .write_event(event)
                    .map_err(|e| DocliError::InvalidDocx {
                        message: e.to_string(),
                    })?;
            }
            Err(e) => {
                return Err(DocliError::InvalidDocx {
                    message: e.to_string(),
                });
            }
        }
        buf.clear();
    }

    Ok(writer.into_inner())
}

/// Collect all valid (< 0x7FFFFFFF) `w:id` values from the XML.
fn collect_valid_w_ids(xml: &[u8]) -> Result<std::collections::HashSet<u64>, DocliError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut ids = std::collections::HashSet::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"w:id" {
                        if let Ok(val_str) = std::str::from_utf8(&attr.value) {
                            if let Ok(v) = val_str.parse::<u64>() {
                                if v < 0x7FFF_FFFF {
                                    ids.insert(v);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => {
                return Err(DocliError::InvalidDocx {
                    message: e.to_string(),
                });
            }
        }
        buf.clear();
    }

    Ok(ids)
}

/// Return a new `BytesStart` where any `w:id` attribute with value >= `0x7FFFFFFF`
/// has been replaced with a fresh sequential id.
fn rewrite_overflow_ids<'a>(
    e: &BytesStart<'a>,
    existing_ids: &std::collections::HashSet<u64>,
    get_next: &mut impl FnMut(&std::collections::HashSet<u64>) -> u64,
) -> Result<BytesStart<'static>, DocliError> {
    let name = String::from_utf8(e.name().as_ref().to_vec()).map_err(|e| {
        DocliError::InvalidDocx {
            message: e.to_string(),
        }
    })?;
    let mut new_e = BytesStart::new(name);

    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == b"w:id" {
            let val_str = std::str::from_utf8(&attr.value).unwrap_or("");
            let val: u64 = val_str.parse().unwrap_or(u64::MAX);
            if val >= 0x7FFF_FFFF {
                let new_id = get_next(existing_ids);
                new_e.push_attribute(("w:id", new_id.to_string().as_str()));
                continue;
            }
        }
        new_e.push_attribute(attr);
    }

    Ok(new_e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_xml_space_preserve_adds_attribute_for_leading_space() {
        let xml = br#"<?xml version="1.0"?><root><w:t> hello</w:t></root>"#;
        let result = ensure_xml_space_preserve(xml).expect("should succeed");
        let out = String::from_utf8(result).unwrap();
        assert!(
            out.contains("xml:space=\"preserve\""),
            "Expected xml:space=preserve in output: {out}"
        );
    }

    #[test]
    fn ensure_xml_space_preserve_adds_attribute_for_trailing_space() {
        let xml = br#"<?xml version="1.0"?><root><w:t>hello </w:t></root>"#;
        let result = ensure_xml_space_preserve(xml).expect("should succeed");
        let out = String::from_utf8(result).unwrap();
        assert!(
            out.contains("xml:space=\"preserve\""),
            "Expected xml:space=preserve in output: {out}"
        );
    }

    #[test]
    fn ensure_xml_space_preserve_does_not_add_when_not_needed() {
        let xml = br#"<?xml version="1.0"?><root><w:t>hello</w:t></root>"#;
        let result = ensure_xml_space_preserve(xml).expect("should succeed");
        let out = String::from_utf8(result).unwrap();
        assert!(
            !out.contains("xml:space"),
            "Should NOT add xml:space when no whitespace: {out}"
        );
    }

    #[test]
    fn ensure_xml_space_preserve_does_not_duplicate_existing_attribute() {
        let xml =
            br#"<?xml version="1.0"?><root><w:t xml:space="preserve"> hello</w:t></root>"#;
        let result = ensure_xml_space_preserve(xml).expect("should succeed");
        let out = String::from_utf8(result).unwrap();
        let count = out.matches("xml:space").count();
        assert_eq!(count, 1, "Should not duplicate xml:space attribute: {out}");
    }

    #[test]
    fn repair_id_overflow_renumbers_overflowing_ids() {
        let xml = br#"<?xml version="1.0"?><root><w:p w:id="2147483647"/></root>"#;
        let result = repair_durable_id_overflow(xml).expect("should succeed");
        let out = String::from_utf8(result).unwrap();
        // The overflow value should be replaced; 2147483647 == 0x7FFFFFFF.
        assert!(
            !out.contains("w:id=\"2147483647\""),
            "Overflow id should be renumbered: {out}"
        );
        assert!(
            out.contains("w:id="),
            "Should still have a w:id attribute: {out}"
        );
    }

    #[test]
    fn repair_id_overflow_preserves_valid_ids() {
        let xml = br#"<?xml version="1.0"?><root><w:p w:id="5"/></root>"#;
        let result = repair_durable_id_overflow(xml).expect("should succeed");
        let out = String::from_utf8(result).unwrap();
        assert!(
            out.contains("w:id=\"5\""),
            "Valid id should be preserved: {out}"
        );
    }

    #[test]
    fn repair_id_overflow_avoids_collision_with_existing_ids() {
        // id=1 is valid and already in use; overflowing id should get 2 (next free).
        let xml =
            br#"<?xml version="1.0"?><root><w:p w:id="1"/><w:p w:id="2147483648"/></root>"#;
        let result = repair_durable_id_overflow(xml).expect("should succeed");
        let out = String::from_utf8(result).unwrap();
        // id=1 should still be present.
        assert!(out.contains("w:id=\"1\""), "Valid id 1 should be kept: {out}");
        // The overflowing id should be renumbered to 2 (since 1 is taken).
        assert!(
            out.contains("w:id=\"2\""),
            "Overflow should be renumbered to 2: {out}"
        );
    }
}
