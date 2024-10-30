use std::time::{Duration, Instant};

pub struct ProcessingStats {
    start_time: Instant,
    processed_chunks: usize,
    total_chunks: usize,
    moving_avg_duration: Option<Duration>,
}

impl ProcessingStats {
    pub fn new(total_chunks: usize) -> Self {
        Self {
            start_time: Instant::now(),
            processed_chunks: 0,
            total_chunks,
            moving_avg_duration: None,
        }
    }

    pub fn update(&mut self, chunk_duration: Duration) {
        self.processed_chunks += 1;

        // Update moving average with exponential weighting
        self.moving_avg_duration = Some(match self.moving_avg_duration {
            Some(avg) => {
                let alpha = 0.2; // Smoothing factor
                avg.mul_f64(1.0 - alpha) + chunk_duration.mul_f64(alpha)
            }
            None => chunk_duration,
        });
    }

    pub fn format_duration(duration: Duration) -> String {
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
        let remaining = self
            .moving_avg_duration
            .map_or(Duration::ZERO, |avg_duration| {
                avg_duration * (self.total_chunks - self.processed_chunks) as u32
            });

        format!(
            "elapsed: {}, remaining: {}",
            Self::format_duration(elapsed),
            Self::format_duration(remaining)
        )
    }
}
