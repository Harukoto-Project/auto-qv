use std::io::Read;
use std::thread;
use std::time::Duration;

use crate::error::{AutoQvError, Result};

const MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const GLOBAL_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS: u32 = 3;
const RETRY_BACKOFF: Duration = Duration::from_secs(1);
const CHUNK_BYTES: usize = 16 * 1024;

pub trait Fetcher: Sync {
    fn get(&self, url: &str, on_progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>>;
}

pub struct HttpFetcher {
    agent: ureq::Agent,
}

impl HttpFetcher {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(GLOBAL_TIMEOUT))
            .build();
        Self {
            agent: config.into(),
        }
    }

    fn get_once(
        &self,
        url: &str,
        on_progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> std::result::Result<Vec<u8>, Attempt> {
        let mut response = self.agent.get(url).call().map_err(Attempt::from_ureq)?;
        let total = response.body().content_length();
        let mut reader = response
            .body_mut()
            .with_config()
            .limit(MAX_BODY_BYTES)
            .reader();

        let mut data = Vec::new();
        let mut chunk = [0u8; CHUNK_BYTES];
        loop {
            let read = reader.read(&mut chunk).map_err(|e| Attempt {
                message: e.to_string(),
                retryable: true,
                not_found: false,
            })?;
            if read == 0 {
                break;
            }
            data.extend_from_slice(&chunk[..read]);
            on_progress(data.len() as u64, total);
        }
        Ok(data)
    }
}

impl Fetcher for HttpFetcher {
    fn get(&self, url: &str, on_progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>> {
        let mut last_error = String::new();
        for attempt in 1..=MAX_ATTEMPTS {
            match self.get_once(url, on_progress) {
                Ok(data) => return Ok(data),
                Err(error) => {
                    if error.not_found {
                        return Err(AutoQvError::NotFound(url.to_string()));
                    }
                    last_error = error.message;
                    if !error.retryable || attempt == MAX_ATTEMPTS {
                        break;
                    }
                    thread::sleep(RETRY_BACKOFF * attempt);
                }
            }
        }
        Err(AutoQvError::Dataset(format!("{url}: {last_error}")))
    }
}

struct Attempt {
    message: String,
    retryable: bool,
    not_found: bool,
}

impl Attempt {
    fn from_ureq(error: ureq::Error) -> Self {
        Self {
            retryable: is_retryable(&error),
            not_found: matches!(error, ureq::Error::StatusCode(404)),
            message: error.to_string(),
        }
    }
}

fn is_retryable(error: &ureq::Error) -> bool {
    match error {
        ureq::Error::StatusCode(code) => *code >= 500 || *code == 429,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_errors_and_rate_limits_are_retried() {
        assert!(is_retryable(&ureq::Error::StatusCode(500)));
        assert!(is_retryable(&ureq::Error::StatusCode(503)));
        assert!(is_retryable(&ureq::Error::StatusCode(429)));
    }

    #[test]
    fn client_errors_are_not_retried() {
        assert!(!is_retryable(&ureq::Error::StatusCode(404)));
        assert!(!is_retryable(&ureq::Error::StatusCode(403)));
    }

    #[test]
    fn network_failures_are_retried() {
        assert!(is_retryable(&ureq::Error::Timeout(ureq::Timeout::Global)));
    }

    #[test]
    fn only_404_is_reported_as_not_found() {
        assert!(Attempt::from_ureq(ureq::Error::StatusCode(404)).not_found);
        assert!(!Attempt::from_ureq(ureq::Error::StatusCode(403)).not_found);
        assert!(!Attempt::from_ureq(ureq::Error::StatusCode(500)).not_found);
    }
}
