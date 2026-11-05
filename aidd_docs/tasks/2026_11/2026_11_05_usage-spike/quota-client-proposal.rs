// Prototype proposé dans la discussion du 2026-11-05.
// Non compilé, non testé, non intégré à l'application. Voir discussion.md.
use std::time::{Duration, SystemTime};

use reqwest::{
    header::{HeaderMap, RETRY_AFTER},
    Client, StatusCode,
};
use serde::Deserialize;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const POLL_INTERVAL: Duration = Duration::from_secs(5 * 60);
const ERROR_BACKOFF: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Deserialize)]
pub struct QuotaWindow {
    pub utilization: Option<f64>,
    pub resets_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Quotas {
    pub five_hour: Option<QuotaWindow>,
    pub seven_day: Option<QuotaWindow>,
}

pub enum QuotaFetch {
    Updated {
        quotas: Quotas,
        observed_at: SystemTime,
        retry_after: Duration,
    },
    RetryAfter(Duration),
    AuthenticationRequired,
}

pub fn usage_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .build()
}

pub async fn fetch_quotas(client: &Client, token: &str) -> QuotaFetch {
    let response = match client
        .get(USAGE_URL)
        .bearer_auth(token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return QuotaFetch::RetryAfter(ERROR_BACKOFF),
    };

    let delay = retry_delay(response.headers());

    match response.status() {
        StatusCode::OK => match response.json::<Quotas>().await {
            Ok(quotas) => QuotaFetch::Updated {
                quotas,
                observed_at: SystemTime::now(),
                retry_after: POLL_INTERVAL,
            },
            Err(_) => QuotaFetch::RetryAfter(ERROR_BACKOFF),
        },
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            QuotaFetch::AuthenticationRequired
        }
        _ => QuotaFetch::RetryAfter(delay),
    }
}

fn retry_delay(headers: &HeaderMap) -> Duration {
    let Some(value) = headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
    else {
        return ERROR_BACKOFF;
    };

    let delay = value
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
        .or_else(|| {
            httpdate::parse_http_date(value)
                .ok()?
                .duration_since(SystemTime::now())
                .ok()
        })
        .unwrap_or(ERROR_BACKOFF);

    delay.max(ERROR_BACKOFF)
}
