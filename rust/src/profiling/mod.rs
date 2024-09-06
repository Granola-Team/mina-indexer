use lazy_static::lazy_static;
use log::info;
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

    fn report_summary(&self) {
        // Calculate total time spent on all operations
        let total_time =
            self.pcb_file_read_duration + self.processing_duration + self.db_write_duration;

        // Avoid division by zero in case no time has been recorded
        if total_time.as_secs_f64() == 0.0 {
            println!("No time recorded for any operation.");
            return;
        }

        // Calculate percentage for each operation
        let pcb_file_read_percentage =
            self.pcb_file_read_duration.as_secs_f64() / total_time.as_secs_f64() * 100.0;
        let processing_percentage =
            self.processing_duration.as_secs_f64() / total_time.as_secs_f64() * 100.0;
        let db_write_percentage =
            self.db_write_duration.as_secs_f64() / total_time.as_secs_f64() * 100.0;

        // Print total times and percentages
        info!("=== Ingestion Profiling ===");
        info!(
            "Total PCB read time: {:?} ({:.2}%)",
            self.pcb_file_read_duration, pcb_file_read_percentage
        );
        info!(
            "Total processing time: {:?} ({:.2}%)",
            self.processing_duration, processing_percentage
        );
        info!(
            "Total DB operations time: {:?} ({:.2}%)",
            self.db_write_duration, db_write_percentage
        );
        info!("Total time spent on all operations: {:?}", total_time);
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

pub fn ingestion_profiling_summary() {
    let profiling = GLOBAL_PROFILING.lock().unwrap();
    profiling.report_summary();
}
