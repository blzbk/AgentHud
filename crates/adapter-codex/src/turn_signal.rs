//! Turn events of a Codex session: `task_started` opens a turn,
//! `task_complete` or `turn_aborted` closes it. Parsing is defensive: any
//! other or malformed line carries no signal.

use domain::TurnSignal;
use serde::Deserialize;

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    payload: Option<Payload>,
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "type", default)]
    kind: Option<String>,
}

/// Turn signal of one session line, `None` when it is not a turn event.
#[must_use]
pub fn parse_line(line: &str) -> Option<TurnSignal> {
    let line: Line = serde_json::from_str(line).ok()?;
    if line.kind.as_deref() != Some("event_msg") {
        return None;
    }
    match line.payload?.kind.as_deref()? {
        "task_started" => Some(TurnSignal::TurnStarted),
        "task_complete" | "turn_aborted" => Some(TurnSignal::TurnEnded),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use domain::TurnSignal;

    use super::parse_line;

    fn event(kind: &str) -> String {
        format!(
            r#"{{"timestamp":"2026-11-03T12:00:00.000Z","type":"event_msg","payload":{{"type":"{kind}"}}}}"#
        )
    }

    #[test]
    fn a_started_task_opens_a_turn() {
        assert_eq!(
            parse_line(&event("task_started")),
            Some(TurnSignal::TurnStarted)
        );
    }

    #[test]
    fn a_complete_or_aborted_task_closes_the_turn() {
        assert_eq!(
            parse_line(&event("task_complete")),
            Some(TurnSignal::TurnEnded)
        );
        assert_eq!(
            parse_line(&event("turn_aborted")),
            Some(TurnSignal::TurnEnded)
        );
    }

    #[test]
    fn other_lines_carry_no_signal() {
        for line in [
            "not json".to_owned(),
            "[]".to_owned(),
            event("token_count"),
            event("item_completed"),
            r#"{"type":"response_item","payload":{"type":"task_started"}}"#.to_owned(),
            r#"{"type":"event_msg","payload":null}"#.to_owned(),
        ] {
            assert_eq!(parse_line(&line), None, "{line}");
        }
    }
}
