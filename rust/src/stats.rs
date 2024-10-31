use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct ProcessingStats {
    start_time: Instant,
    processed_chunks: AtomicUsize,
    total_chunks: usize,
    moving_avg_duration: Mutex<Option<Duration>>,
}

impl ProcessingStats {
    pub fn new(total_chunks: usize) -> Self {
        Self {
            start_time: Instant::now(),
            processed_chunks: AtomicUsize::new(0),
            total_chunks,
            moving_avg_duration: Mutex::new(None),
        }
    }

    pub async fn update(&self, chunk_duration: Duration) {
        self.processed_chunks.fetch_add(1, Ordering::SeqCst);

        // Update moving average with exponential weighting
        let mut avg = self.moving_avg_duration.lock().await;
        *avg = Some(match *avg {
            Some(avg_dur) => {
                let alpha = 0.2; // Smoothing factor
                avg_dur.mul_f64(1.0 - alpha) + chunk_duration.mul_f64(alpha)
            }
            None => chunk_duration,
        });
    }

    fn format_duration(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;

        if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }

    pub async fn get_stats(&self) -> String {
        let elapsed = self.start_time.elapsed();
        let processed = self.processed_chunks.load(Ordering::Relaxed);
        let avg_duration = self.moving_avg_duration.lock().await;

        let remaining = avg_duration.map_or(Duration::ZERO, |avg_dur| {
            avg_dur * (self.total_chunks - processed) as u32
        });

        format!(
            "Progress: {}/{} chunks, elapsed: {}, remaining: {}",
            processed,
            self.total_chunks,
            Self::format_duration(elapsed),
            Self::format_duration(remaining)
        )
    }
}
