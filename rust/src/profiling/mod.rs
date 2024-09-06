use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::Duration;

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
        println!("Total PCB read time: {:?}", self.pcb_file_read_duration);
        println!("Total processing time: {:?}", self.processing_duration);
        println!("Total DB operations time: {:?}", self.db_write_duration);
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

pub fn report_summary() {
    let profiling = GLOBAL_PROFILING.lock().unwrap();
    profiling.report_summary();
}
