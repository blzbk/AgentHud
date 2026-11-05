//! Options the user toggles from the tray, saved between launches.

use serde::{Deserialize, Serialize};

/// Which remote quota sources may be polled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    /// Claude quotas from the OAuth endpoint, the degraded mode Anthropic
    /// does not allow (ADR 0002). Off unless the user enables it.
    pub claude_oauth: bool,
    /// Codex quotas from `codex app-server`, the official client. On by
    /// default.
    pub codex_app_server: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            claude_oauth: false,
            codex_app_server: true,
        }
    }
}

impl Options {
    /// Saved form of the options.
    ///
    /// # Errors
    ///
    /// Fails only if serialization itself fails, which plain flags cannot.
    pub fn to_json(self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self)
    }

    /// Reads options saved by [`Options::to_json`]. A missing field keeps its
    /// default, an unknown one is ignored.
    ///
    /// # Errors
    ///
    /// Fails if `json` is not an options object.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn oauth_is_off_and_app_server_on_by_default() {
        assert_eq!(
            Options::default(),
            Options {
                claude_oauth: false,
                codex_app_server: true
            }
        );
    }

    #[test]
    fn a_missing_field_keeps_its_default() {
        let options =
            Options::from_json(r#"{"claudeOauth":true,"later":1}"#).expect("options should parse");

        assert_eq!(
            options,
            Options {
                claude_oauth: true,
                codex_app_server: true
            }
        );
        assert_eq!(
            Options::from_json("{}").expect("options should parse"),
            Options::default()
        );
    }

    #[test]
    fn rejects_what_is_not_an_options_object() {
        for json in ["", "true", "42", r#"{"claudeOauth":"yes"}"#] {
            assert!(Options::from_json(json).is_err(), "{json}");
        }
    }

    #[test]
    fn round_trips_through_json() {
        let options = Options {
            claude_oauth: true,
            codex_app_server: false,
        };

        let saved = options.to_json().expect("options should serialize");

        assert_eq!(
            Options::from_json(&saved).expect("options should parse"),
            options
        );
    }
}
