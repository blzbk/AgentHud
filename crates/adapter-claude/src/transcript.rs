//! Defensive parsing of the lines of a Claude Code transcript
//! (`projects/<project>/<session>.jsonl`). Only `assistant` and `user` lines
//! matter; every other type (`attachment`, `last-prompt`, `mode`, `system`,
//! ...) is metadata and is ignored, as is any malformed line.

use domain::{EpochSeconds, TurnSignal};
use serde::Deserialize;

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(rename = "requestId", default)]
    request_id: Option<String>,
    #[serde(rename = "isMeta", default)]
    is_meta: Option<bool>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<RawUsage>,
    #[serde(default)]
    content: Option<Content>,
}

#[derive(Deserialize)]
struct RawUsage {
    #[serde(rename = "input_tokens", default)]
    input: Option<u64>,
    #[serde(rename = "output_tokens", default)]
    output: Option<u64>,
    #[serde(rename = "cache_creation_input_tokens", default)]
    cache_creation: Option<u64>,
    #[serde(rename = "cache_read_input_tokens", default)]
    cache_read: Option<u64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Text(String),
    Parts(Vec<Part>),
}

#[derive(Deserialize)]
struct Part {
    #[serde(default)]
    text: Option<String>,
}

/// Tokens of one assistant message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEntry {
    /// `message.id` and `requestId`: a message written on several lines, or
    /// copied into a resumed session, shares this key and counts once.
    pub key: Option<String>,
    pub timestamp: EpochSeconds,
    /// Input, output, cache creation and cache read tokens.
    pub tokens: u64,
}

/// Turn state carried by a line, and the folder the session runs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnLine {
    pub signal: TurnSignal,
    pub cwd: Option<String>,
}

/// Usage of an `assistant` line, `None` for any other line or when the
/// usage or the timestamp is missing.
#[must_use]
pub fn usage_entry(line: &str) -> Option<UsageEntry> {
    let line: Line = serde_json::from_str(line).ok()?;
    if line.kind.as_deref() != Some("assistant") {
        return None;
    }
    let timestamp = adapter_fs::parse_utc_timestamp(line.timestamp.as_deref()?)?;
    let message = line.message?;
    let usage = message.usage?;
    let tokens = [
        usage.input,
        usage.output,
        usage.cache_creation,
        usage.cache_read,
    ]
    .into_iter()
    .flatten()
    .fold(0_u64, u64::saturating_add);
    let key = message
        .id
        .zip(line.request_id)
        .map(|(id, request)| format!("{id}:{request}"));
    Some(UsageEntry {
        key,
        timestamp,
        tokens,
    })
}

/// Turn signal of a line, `None` when the line is metadata.
///
/// - `assistant` ending on `tool_use` (or still streaming): the turn goes on.
/// - `assistant` with any other stop reason: the turn is over.
/// - `user`: a prompt or a tool result starts or feeds a turn, except an
///   interruption or the output of a local command, which ends it.
#[must_use]
pub fn turn_line(line: &str) -> Option<TurnLine> {
    let line: Line = serde_json::from_str(line).ok()?;
    let message = line.message?;
    let signal = match line.kind.as_deref()? {
        "assistant" => match message.stop_reason.as_deref() {
            None | Some("tool_use") => TurnSignal::TurnStarted,
            Some(_) => TurnSignal::TurnEnded,
        },
        "user" if line.is_meta == Some(true) => return None,
        "user" if message.content.as_ref().is_some_and(ends_the_turn) => TurnSignal::TurnEnded,
        "user" => TurnSignal::TurnStarted,
        _ => return None,
    };
    Some(TurnLine {
        signal,
        cwd: line.cwd,
    })
}

/// Whether a user message closes the turn instead of feeding it.
fn ends_the_turn(content: &Content) -> bool {
    const ENDING_PREFIXES: [&str; 3] = [
        "[Request interrupted by user",
        "<local-command-stdout>",
        "<local-command-stderr>",
    ];
    let first_text = match content {
        Content::Text(text) => Some(text.as_str()),
        Content::Parts(parts) => parts.iter().find_map(|part| part.text.as_deref()),
    };
    first_text.is_some_and(|text| {
        ENDING_PREFIXES
            .iter()
            .any(|prefix| text.trim_start().starts_with(prefix))
    })
}

#[cfg(test)]
mod tests {
    use domain::TurnSignal;

    use super::{turn_line, usage_entry, UsageEntry};

    fn assistant(stop_reason: &str) -> String {
        format!(
            r#"{{"type":"assistant","timestamp":"2026-11-03T12:00:00.000Z","requestId":"req_1","cwd":"/workspace/project","message":{{"id":"msg_1","stop_reason":{stop_reason},"usage":{{"input_tokens":1,"output_tokens":2,"cache_creation_input_tokens":3,"cache_read_input_tokens":4}},"content":[]}}}}"#
        )
    }

    fn user(content: &str) -> String {
        format!(
            r#"{{"type":"user","timestamp":"2026-11-03T12:00:00.000Z","message":{{"role":"user","content":{content}}}}}"#
        )
    }

    fn signal(line: &str) -> Option<TurnSignal> {
        turn_line(line).map(|turn| turn.signal)
    }

    #[test]
    fn sums_the_four_token_counts_of_an_assistant_line() {
        assert_eq!(
            usage_entry(&assistant(r#""end_turn""#)),
            Some(UsageEntry {
                key: Some("msg_1:req_1".to_owned()),
                timestamp: 1_793_707_200,
                tokens: 10,
            })
        );
    }

    #[test]
    fn a_line_without_ids_has_no_key() {
        let line = r#"{"type":"assistant","timestamp":"2026-11-03T12:00:00Z","message":{"usage":{"output_tokens":7}}}"#;

        assert_eq!(
            usage_entry(line),
            Some(UsageEntry {
                key: None,
                timestamp: 1_793_707_200,
                tokens: 7
            })
        );
    }

    #[test]
    fn only_dated_assistant_lines_carry_usage() {
        for line in [
            "not json".to_owned(),
            user(r#""prompt""#),
            r#"{"type":"assistant","message":{"usage":{"output_tokens":7}}}"#.to_owned(),
            r#"{"type":"assistant","timestamp":"2026-11-03T12:00:00Z","message":{}}"#.to_owned(),
            r#"{"type":"last-prompt","lastPrompt":"prompt"}"#.to_owned(),
        ] {
            assert_eq!(usage_entry(&line), None, "{line}");
        }
    }

    #[test]
    fn an_assistant_line_ends_the_turn_unless_it_calls_a_tool() {
        assert_eq!(
            signal(&assistant(r#""end_turn""#)),
            Some(TurnSignal::TurnEnded)
        );
        assert_eq!(
            signal(&assistant(r#""max_tokens""#)),
            Some(TurnSignal::TurnEnded)
        );
        assert_eq!(
            signal(&assistant(r#""tool_use""#)),
            Some(TurnSignal::TurnStarted)
        );
        assert_eq!(signal(&assistant("null")), Some(TurnSignal::TurnStarted));
    }

    #[test]
    fn a_prompt_or_a_tool_result_feeds_the_turn() {
        assert_eq!(signal(&user(r#""prompt""#)), Some(TurnSignal::TurnStarted));
        assert_eq!(
            signal(&user(
                r#"[{"type":"tool_result","tool_use_id":"t","content":"output"}]"#
            )),
            Some(TurnSignal::TurnStarted)
        );
    }

    #[test]
    fn an_interruption_or_a_local_command_ends_the_turn() {
        assert_eq!(
            signal(&user(
                r#"[{"type":"text","text":"[Request interrupted by user]"}]"#
            )),
            Some(TurnSignal::TurnEnded)
        );
        assert_eq!(
            signal(&user(
                r#""<local-command-stdout>done</local-command-stdout>""#
            )),
            Some(TurnSignal::TurnEnded)
        );
    }

    #[test]
    fn metadata_lines_carry_no_signal() {
        for line in [
            "not json",
            r#"{"type":"user","isMeta":true,"message":{"role":"user","content":"caveat"}}"#,
            r#"{"type":"system","subtype":"turn_duration","message":{}}"#,
            r#"{"type":"attachment","attachment":{}}"#,
            r#"{"type":"permission-mode","permissionMode":"default"}"#,
        ] {
            assert_eq!(signal(line), None, "{line}");
        }
    }

    #[test]
    fn keeps_the_working_folder() {
        assert_eq!(
            turn_line(&assistant(r#""end_turn""#)).and_then(|turn| turn.cwd),
            Some("/workspace/project".to_owned())
        );
    }
}
