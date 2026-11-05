//! The login Claude Code keeps in `<.claude>/.credentials.json`. Read again
//! for each request of the degraded OAuth mode, never copied nor stored, and
//! never logged (ADR 0002).

use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::Deserialize;

/// File of the Claude Code login, under each `.claude` folder.
const CREDENTIALS_FILE: &str = ".credentials.json";

/// An OAuth access token. Its `Debug` output hides it.
pub struct AccessToken(String);

impl AccessToken {
    /// The token, only to build the `Authorization` header.
    pub(crate) fn secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AccessToken(***)")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Credentials {
    #[serde(default)]
    claude_ai_oauth: Option<OAuth>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuth {
    #[serde(default)]
    access_token: Option<String>,
}

/// Access token of the first root holding a Claude Code login, in root
/// order. Unreadable files are skipped; their content is never logged.
pub(crate) fn access_token(roots: &[PathBuf]) -> Option<AccessToken> {
    roots.iter().find_map(|root| {
        let path = root.join(CREDENTIALS_FILE);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                if error.kind() != io::ErrorKind::NotFound {
                    log::debug!("cannot read {}: {}", path.display(), error.kind());
                }
                return None;
            }
        };
        let token = serde_json::from_str::<Credentials>(&text)
            .ok()?
            .claude_ai_oauth?
            .access_token?;
        (!token.trim().is_empty()).then(|| AccessToken(token.trim().to_owned()))
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{access_token, CREDENTIALS_FILE};

    #[test]
    fn reads_the_token_of_the_first_root_with_a_login() {
        let empty = tempfile::tempdir().expect("temporary folder should be writable");
        let logged = tempfile::tempdir().expect("temporary folder should be writable");
        fs::write(
            logged.path().join(CREDENTIALS_FILE),
            r#"{"claudeAiOauth":{"accessToken":"fake-token","refreshToken":"fake-refresh","expiresAt":1}}"#,
        )
        .expect("file should be writable");

        let token = access_token(&[empty.path().to_path_buf(), logged.path().to_path_buf()]);

        assert_eq!(
            token.map(|token| token.secret().to_owned()),
            Some("fake-token".to_owned())
        );
    }

    #[test]
    fn has_no_token_without_a_usable_login() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");
        assert!(access_token(&[root.path().to_path_buf()]).is_none());

        for text in [
            "",
            "{}",
            r#"{"claudeAiOauth":null}"#,
            r#"{"claudeAiOauth":{"accessToken":"  "}}"#,
        ] {
            fs::write(root.path().join(CREDENTIALS_FILE), text).expect("file should be writable");
            assert!(
                access_token(&[root.path().to_path_buf()]).is_none(),
                "{text}"
            );
        }
    }

    #[test]
    fn debug_output_hides_the_token() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");
        fs::write(
            root.path().join(CREDENTIALS_FILE),
            r#"{"claudeAiOauth":{"accessToken":"fake-token"}}"#,
        )
        .expect("file should be writable");

        let token = access_token(&[root.path().to_path_buf()]).expect("token should be read");

        assert_eq!(format!("{token:?}"), "AccessToken(***)");
    }
}
