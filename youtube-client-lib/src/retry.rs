//! # Exponential Backoff and Retry Primitives

use crate::error::YoutubeError;
use rand::Rng;
use std::time::Duration;

/// Retry an asynchronous Google YouTube API call with exponential backoff and random jitter.
///
/// Automatically classifies transient HTTP 5xx errors and 429 rate limit errors as retryable,
/// while promptly identifying 403 quota breaches and returning [`YoutubeError::QuotaExceeded`].
pub async fn retry_api_call<F, Fut, T>(f: F) -> std::result::Result<T, YoutubeError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, google_youtube3::Error>>,
{
    let mut attempts = 0;
    let mut delay = Duration::from_millis(500);

    loop {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                attempts += 1;

                // Check if the error indicates a quota limit breach
                if let google_youtube3::Error::Failure(ref resp) = e {
                    if resp.status().as_u16() == 403 {
                        let status_str = format!("{resp:?}");
                        if status_str.contains("quotaExceeded")
                            || status_str.contains("Quota Exceeded")
                            || status_str.contains("403")
                        {
                            return Err(YoutubeError::QuotaExceeded(status_str));
                        }
                    }
                }

                if attempts >= 3 {
                    return Err(YoutubeError::Api(Box::new(e)));
                }

                let is_retryable = match &e {
                    google_youtube3::Error::HttpError(_) => true,
                    google_youtube3::Error::Failure(resp) => {
                        let status = resp.status();
                        status.is_server_error() || status.as_u16() == 429
                    }
                    _ => false,
                };

                if !is_retryable {
                    return Err(YoutubeError::Api(Box::new(e)));
                }

                // Exponential backoff with jitter: delay * (0.5 to 1.5)
                let jitter: f64 = rand::thread_rng().gen_range(0.5..1.5);
                let wait_duration = delay.mul_f64(jitter);

                tokio::time::sleep(wait_duration).await;
                delay *= 2;
            }
        }
    }
}
