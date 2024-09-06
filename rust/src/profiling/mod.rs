use lazy_static::lazy_static;
use std::{sync::Mutex, time::Duration};

// Define the Profiling struct
struct IngestionProfiling {
    pcb_file_read_duration: Duration,
    processing_duration: Duration,
    db_write_duration: Duration,
}

// Implement methods for the Profiling struct
impl IngestionProfiling {
    fn new() -> Self {
        IngestionProfiling {
            pcb_file_read_duration: Duration::new(0, 0),
            processing_duration: Duration::new(0, 0),
            db_write_duration: Duration::new(0, 0),
        }
    }

    fn record_file_read(&mut self, duration: Duration) {
        self.pcb_file_read_duration += duration;
    }

    fn record_processing(&mut self, duration: Duration) {
        self.processing_duration += duration;
    }

    fn record_db_write(&mut self, duration: Duration) {
        self.db_write_duration += duration;
    }

    fn report_summary(&self) -> [String; 4] {
        // Calculate total time spent on all operations
        let total_time =
            self.pcb_file_read_duration + self.processing_duration + self.db_write_duration;

        // Handle the case where no time has been recorded
        if total_time.as_secs_f64() == 0.0 {
            return [
                "No time recorded for any operation.".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ];
        }

        // Calculate percentage for each operation
        let pcb_file_read_percentage =
            self.pcb_file_read_duration.as_secs_f64() / total_time.as_secs_f64() * 100.0;
        let processing_percentage =
            self.processing_duration.as_secs_f64() / total_time.as_secs_f64() * 100.0;
        let db_write_percentage =
            self.db_write_duration.as_secs_f64() / total_time.as_secs_f64() * 100.0;

        // Prepare summary strings
        let pcb_read_str = format!(
            "Total PCB read time: {:?} ({:.2}%)",
            self.pcb_file_read_duration, pcb_file_read_percentage
        );
        let processing_str = format!(
            "Total processing time: {:?} ({:.2}%)",
            self.processing_duration, processing_percentage
        );
        let db_write_str = format!(
            "Total DB operations time: {:?} ({:.2}%)",
            self.db_write_duration, db_write_percentage
        );
        let total_time_str = format!("Total time spent on all operations: {:?}", total_time);

        [pcb_read_str, processing_str, db_write_str, total_time_str]
    }
}

// Create the singleton Profiling object using lazy_static
lazy_static! {
    static ref GLOBAL_PROFILING: Mutex<IngestionProfiling> = Mutex::new(IngestionProfiling::new());
}

// Function to access and mutate the singleton Profiling object
pub fn aggregate_read_duration(duration: Duration) {
    let mut profiling = GLOBAL_PROFILING.lock().unwrap();
    profiling.record_file_read(duration);
}

pub fn aggregate_processing_duration(duration: Duration) {
    let mut profiling = GLOBAL_PROFILING.lock().unwrap();
    profiling.record_processing(duration);
}

pub fn aggregate_db_operation_duration(duration: Duration) {
    let mut profiling = GLOBAL_PROFILING.lock().unwrap();
    profiling.record_db_write(duration);
}

pub fn ingestion_profiling_summary() -> [String; 4] {
    let profiling = GLOBAL_PROFILING.lock().unwrap();
    profiling.report_summary()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_initialization() {
        let profiling = IngestionProfiling::new();
        assert_eq!(profiling.pcb_file_read_duration, Duration::new(0, 0));
        assert_eq!(profiling.processing_duration, Duration::new(0, 0));
        assert_eq!(profiling.db_write_duration, Duration::new(0, 0));
    }

    #[test]
    fn test_aggregate_read_duration() {
        let mut profiling = IngestionProfiling::new();
        profiling.record_file_read(Duration::new(2, 0));
        assert_eq!(profiling.pcb_file_read_duration, Duration::new(2, 0));

        profiling.record_file_read(Duration::new(1, 0));
        assert_eq!(profiling.pcb_file_read_duration, Duration::new(3, 0));
    }

    #[test]
    fn test_aggregate_processing_duration() {
        let mut profiling = IngestionProfiling::new();
        profiling.record_processing(Duration::new(5, 0));
        assert_eq!(profiling.processing_duration, Duration::new(5, 0));

        profiling.record_processing(Duration::new(2, 0));
        assert_eq!(profiling.processing_duration, Duration::new(7, 0));
    }

    #[test]
    fn test_aggregate_db_operation_duration() {
        let mut profiling = IngestionProfiling::new();
        profiling.record_db_write(Duration::new(3, 0));
        assert_eq!(profiling.db_write_duration, Duration::new(3, 0));

        profiling.record_db_write(Duration::new(1, 500_000_000)); // 1.5 seconds
        assert_eq!(profiling.db_write_duration, Duration::new(4, 500_000_000));
    }

    #[test]
    fn test_report_summary() {
        let profiling = IngestionProfiling {
            pcb_file_read_duration: Duration::new(2, 0),
            processing_duration: Duration::new(5, 0),
            db_write_duration: Duration::new(3, 0),
        };

        let summary = profiling.report_summary();

        assert_eq!(summary[0], "Total PCB read time: 2s (20.00%)");
        assert_eq!(summary[1], "Total processing time: 5s (50.00%)");
        assert_eq!(summary[2], "Total DB operations time: 3s (30.00%)");
        assert_eq!(summary[3], "Total time spent on all operations: 10s");
    }

    #[test]
    fn test_global_profiling_aggregate() {
        // Test the global profiling aggregation functions
        {
            let mut profiling = GLOBAL_PROFILING.lock().unwrap();
            profiling.pcb_file_read_duration = Duration::new(0, 0);
            profiling.processing_duration = Duration::new(0, 0);
            profiling.db_write_duration = Duration::new(0, 0);
        }

        // Test read duration
        aggregate_read_duration(Duration::new(2, 0));
        {
            let profiling = GLOBAL_PROFILING.lock().unwrap();
            assert_eq!(profiling.pcb_file_read_duration, Duration::new(2, 0));
        }

        // Test processing duration
        aggregate_processing_duration(Duration::new(3, 0));
        {
            let profiling = GLOBAL_PROFILING.lock().unwrap();
            assert_eq!(profiling.processing_duration, Duration::new(3, 0));
        }

        // Test DB write duration
        aggregate_db_operation_duration(Duration::new(4, 0));
        {
            let profiling = GLOBAL_PROFILING.lock().unwrap();
            assert_eq!(profiling.db_write_duration, Duration::new(4, 0));
        }
    }
}
