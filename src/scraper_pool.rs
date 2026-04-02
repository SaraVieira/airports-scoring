use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// A pool of Google scraper instances. Each scraper can handle one job at a time.
/// The pool hands out the next available scraper URL, blocking if all are busy.
#[derive(Clone)]
pub struct ScraperPool {
    inner: Arc<Mutex<Vec<ScraperInstance>>>,
    notify: Arc<tokio::sync::Notify>,
}

struct ScraperInstance {
    pub url: String,
    pub busy: bool,
}

impl ScraperPool {
    /// Create a pool from a comma-separated list of URLs.
    /// Falls back to GOOGLE_SCRAPER_URL (single instance) if GOOGLE_SCRAPER_URLS is not set.
    pub fn from_env() -> Option<Self> {
        let urls_str = std::env::var("GOOGLE_SCRAPER_URLS")
            .or_else(|_| std::env::var("GOOGLE_SCRAPER_URL").map(|u| u.to_string()))
            .ok()?;

        let urls: Vec<String> = urls_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if urls.is_empty() {
            return None;
        }

        info!(count = urls.len(), "Scraper pool initialized");

        let instances = urls
            .into_iter()
            .map(|url| ScraperInstance { url, busy: false })
            .collect();

        Some(Self {
            inner: Arc::new(Mutex::new(instances)),
            notify: Arc::new(tokio::sync::Notify::new()),
        })
    }

    /// Acquire an available scraper. Blocks until one is free.
    /// Returns a guard that releases the scraper when dropped.
    pub async fn acquire(&self) -> ScraperGuard {
        loop {
            {
                let mut instances = self.inner.lock().await;
                if let Some(instance) = instances.iter_mut().find(|i| !i.busy) {
                    instance.busy = true;
                    let url = instance.url.clone();
                    info!(url = %url, "Acquired scraper");
                    return ScraperGuard {
                        url,
                        pool: self.clone(),
                    };
                }
            }
            // All busy — wait for a release notification
            self.notify.notified().await;
        }
    }

    /// Release a scraper back to the pool.
    async fn release(&self, url: &str) {
        let mut instances = self.inner.lock().await;
        if let Some(instance) = instances.iter_mut().find(|i| i.url == url) {
            instance.busy = false;
            info!(url = %url, "Released scraper");
        }
        drop(instances);
        self.notify.notify_waiters();
    }

    /// Number of scrapers in the pool.
    pub async fn size(&self) -> usize {
        self.inner.lock().await.len()
    }
}

/// Guard that holds an acquired scraper. Releases it when dropped.
pub struct ScraperGuard {
    pub url: String,
    pool: ScraperPool,
}

impl Drop for ScraperGuard {
    fn drop(&mut self) {
        let pool = self.pool.clone();
        let url = self.url.clone();
        tokio::spawn(async move {
            pool.release(&url).await;
        });
    }
}
