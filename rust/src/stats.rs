use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

pub struct ProcessingStats {
    start_time: Instant,
    processed_chunks: AtomicUsize,
    total_chunks: usize,
}

impl ProcessingStats {
    pub fn new(total_chunks: usize) -> Self {
        Self {
            start_time: Instant::now(),
            processed_chunks: AtomicUsize::new(0),
            total_chunks,
        }
    }

    pub fn update(&self) {
        self.processed_chunks.fetch_add(1, Ordering::SeqCst);
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

    pub fn get_stats(&self) -> String {
        let elapsed = self.start_time.elapsed();
        let processed = self.processed_chunks.load(Ordering::Relaxed);

        let percentage = (processed as f64 / self.total_chunks as f64 * 100.0).round() as u32;

        let remaining = if processed > 0 {
            let avg_time_per_chunk = elapsed.div_f64(processed as f64);
            avg_time_per_chunk.mul_f64((self.total_chunks - processed) as f64)
        } else {
            Duration::ZERO
        };

        format!(
            "Progress: {}/{} chunks ({}%), elapsed: {}, remaining: {}",
            processed,
            self.total_chunks,
            percentage,
            Self::format_duration(elapsed),
            Self::format_duration(remaining)
        )
    }

    pub fn processed_count(&self) -> usize {
        self.processed_chunks.load(Ordering::SeqCst)
    }
}
