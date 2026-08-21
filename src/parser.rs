use anyhow::{Context, Result};
use scraper::{Html, Selector};

const REGISTRATION_CLOSED_TEXT: &str = "参加を締め切りました";

#[derive(Debug, Clone, PartialEq)]
pub struct EventStatus {
    pub current: usize,
    pub limit: Option<usize>,
    pub closed: bool,
}

impl EventStatus {
    pub fn has_free_slot(&self) -> bool {
        match self.limit {
            Some(limit) => !self.closed && self.current < limit,
            None => false,
        }
    }

    pub fn is_joinable(&self) -> bool {
        !self.closed && self.limit.is_none_or(|limit| self.current < limit)
    }
}

/// Parse the Twipla event HTML to extract participant information
pub fn parse_event_html(html: &str) -> Result<EventStatus> {
    let document = Html::parse_document(html);

    let closed = is_registration_closed(&document);

    // Select all divs with class "member_list"
    let div_selector = Selector::parse("div.member_list").unwrap();

    for div in document.select(&div_selector) {
        let text = div.text().collect::<String>();

        // Look for the div that starts with "参加者"
        if text.trim().starts_with("参加者") {
            let mut status = parse_participant_text(&text)?;
            status.closed = closed;
            return Ok(status);
        }
    }

    anyhow::bail!("Could not find participant information in HTML")
}

fn is_registration_closed(document: &Html) -> bool {
    let join_div_selector = Selector::parse("div.join_div").unwrap();

    document.select(&join_div_selector).any(|div| {
        div.text()
            .collect::<String>()
            .contains(REGISTRATION_CLOSED_TEXT)
    })
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
            anyhow::bail!("Unexpected format with limit: {content}");
        }

        let current = parse_number(parts[0])?;
        let limit = parse_number(parts[1])?;

        Ok(EventStatus {
            current,
            limit: Some(limit),
            closed: false,
        })
    } else {
        // Format: "90人"
        let current = parse_number(content)?;

        Ok(EventStatus {
            current,
            limit: None,
            closed: false,
        })
    }
}

fn parse_number(s: &str) -> Result<usize> {
    // Remove "人" suffix and parse
    let num_str = s.trim().trim_end_matches('人');
    num_str
        .parse::<usize>()
        .with_context(|| format!("Failed to parse number from: {s}"))
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
        assert!(!status.closed);
        assert!(!status.has_free_slot());
    }

    #[test]
    fn test_parse_full_event_real_markup() {
        // Mirrors https://twipla.jp/events/738277 (open but full, 110/110)
        let html = r#"
            <div class="round_border join_div"><strong>このイベントに参加しますか？</strong><br />まず<a href='/accounts/login/events/738277'>X(Twitter)アカウントでログイン</a>する</div>
            <div class='clear_both'></div>
            <div class='float_left member_list round_border'>参加者 (110人／定員110人) <br/><ul><li>user1</li></ul></div>
            <div class='float_left member_list round_border'>興味あり (47人) <br/><ul></ul></div>
            <div class='float_left member_list round_border'>不参加 (2人) <br/><ul></ul></div>
        "#;

        let status = parse_event_html(html).unwrap();
        assert_eq!(status.current, 110);
        assert_eq!(status.limit, Some(110));
        assert!(!status.closed);
        assert!(!status.has_free_slot());
    }

    #[test]
    fn test_parse_closed_event_real_markup() {
        // Mirrors https://twipla.jp/events/739233 (registration closed)
        let html = r#"
            <div class="round_border join_div"><span style="color:#e00">参加を締め切りました</span></div>
            <div class='clear_both'></div>
            <div class='float_left member_list round_border'>参加者 (1人) <br/><ul><li>user1</li></ul></div>
            <div class='float_left member_list round_border'>興味あり (0人) <br/><ul></ul></div>
            <div class='float_left member_list round_border'>不参加 (0人) <br/><ul></ul></div>
        "#;

        let status = parse_event_html(html).unwrap();
        assert_eq!(status.current, 1);
        assert_eq!(status.limit, None);
        assert!(status.closed);
        assert!(!status.has_free_slot());
    }

    #[test]
    fn test_closed_event_with_free_slots_does_not_alert() {
        let html = r#"
            <div class="round_border join_div"><span>参加を締め切りました</span></div>
            <div class="member_list">
                参加者 (50人／定員100人)
            </div>
        "#;

        let status = parse_event_html(html).unwrap();
        assert_eq!(status.current, 50);
        assert_eq!(status.limit, Some(100));
        assert!(status.closed);
        assert!(!status.has_free_slot());
    }

    #[test]
    fn test_is_joinable() {
        let status = |current, limit, closed| EventStatus {
            current,
            limit,
            closed,
        };

        assert!(status(5, None, false).is_joinable());
        assert!(status(5, Some(10), false).is_joinable());
        assert!(!status(10, Some(10), false).is_joinable());
        assert!(!status(5, None, true).is_joinable());
        assert!(!status(5, Some(10), true).is_joinable());
    }

    #[test]
    fn test_closed_text_outside_join_div_is_ignored() {
        // The event description is user-authored text and could mention the
        // phrase; only div.join_div is authoritative.
        let html = r#"
            <div class="desc">前回は参加を締め切りました。</div>
            <div class="round_border join_div"><strong>このイベントに参加しますか？</strong></div>
            <div class="member_list">
                参加者 (50人／定員100人)
            </div>
        "#;

        let status = parse_event_html(html).unwrap();
        assert!(!status.closed);
        assert!(status.has_free_slot());
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
