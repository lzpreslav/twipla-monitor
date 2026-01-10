use anyhow::{Context, Result};
use scraper::{Html, Selector};

#[derive(Debug, Clone, PartialEq)]
pub struct EventStatus {
    pub current: usize,
    pub limit: Option<usize>,
}

impl EventStatus {
    pub fn has_free_slot(&self) -> bool {
        match self.limit {
            Some(limit) => self.current < limit,
            None => false,
        }
    }
}

/// Parse the Twipla event HTML to extract participant information
pub fn parse_event_html(html: &str) -> Result<EventStatus> {
    let document = Html::parse_document(html);

    // Select all divs with class "member_list"
    let div_selector = Selector::parse("div.member_list").unwrap();

    for div in document.select(&div_selector) {
        let text = div.text().collect::<Vec<_>>().concat();

        // Look for the div that starts with "参加者"
        if text.trim().starts_with("参加者") {
            return parse_participant_text(&text);
        }
    }

    anyhow::bail!("Could not find participant information in HTML")
}

fn parse_participant_text(text: &str) -> Result<EventStatus> {
    // Expected formats:
    // "参加者 (100人／定員100人)" - with limit
    // "参加者 (90人)" - without limit

    let text = text.trim();

    // Extract the part inside parentheses
    let start = text.find('(').context("No opening parenthesis found")?;
    let end = text.find(')').context("No closing parenthesis found")?;
    let content = &text[start + 1..end];

    // Check if there's a limit (contains ／定員)
    if content.contains("／定員") {
        // Format: "100人／定員100人"
        let parts: Vec<&str> = content.split("／定員").collect();
        if parts.len() != 2 {
            anyhow::bail!("Unexpected format with limit: {}", content);
        }

        let current = parse_number(parts[0])?;
        let limit = parse_number(parts[1])?;

        Ok(EventStatus {
            current,
            limit: Some(limit),
        })
    } else {
        // Format: "90人"
        let current = parse_number(content)?;

        Ok(EventStatus {
            current,
            limit: None,
        })
    }
}

fn parse_number(s: &str) -> Result<usize> {
    // Remove "人" suffix and parse
    let num_str = s.trim().trim_end_matches('人');
    num_str
        .parse::<usize>()
        .with_context(|| format!("Failed to parse number from: {}", s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_limit_full() {
        let html = r#"
            <div class="member_list">
                参加者 (100人／定員100人)
                <ul><li>user1</li></ul>
            </div>
        "#;

        let status = parse_event_html(html).unwrap();
        assert_eq!(status.current, 100);
        assert_eq!(status.limit, Some(100));
        assert!(!status.has_free_slot());
    }

    #[test]
    fn test_parse_with_limit_available() {
        let html = r#"
            <div class="member_list">
                参加者 (50人／定員100人)
            </div>
        "#;

        let status = parse_event_html(html).unwrap();
        assert_eq!(status.current, 50);
        assert_eq!(status.limit, Some(100));
        assert!(status.has_free_slot());
    }

    #[test]
    fn test_parse_without_limit() {
        let html = r#"
            <div class="member_list">
                参加者 (90人)
            </div>
        "#;

        let status = parse_event_html(html).unwrap();
        assert_eq!(status.current, 90);
        assert_eq!(status.limit, None);
        assert!(!status.has_free_slot());
    }

    #[test]
    fn test_parse_ignores_other_member_lists() {
        let html = r#"
            <div class="float_left member_list round_border">
                興味あり (10人)
            </div>
            <div class="float_left member_list round_border">
                参加者 (100人／定員100人)
            </div>
        "#;

        let status = parse_event_html(html).unwrap();
        assert_eq!(status.current, 100);
        assert_eq!(status.limit, Some(100));
    }
}
