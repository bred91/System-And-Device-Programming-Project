use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use sysinfo::{Pid, System};

use chrono::Local;

/// A logger for recording CPU usage and backup details to a file.
#[derive(Clone)]
pub struct Logger {
    log_file_path: String,
}

impl Logger {
    /// Creates a new `Logger` instance.
    ///
    /// # Arguments
    ///
    /// * `log_file_path` - The path where the log file will be created.
    /// * `is_cpu` - A boolean indicating whether the logger is for CPU usage or backup details.
    ///
    /// # Returns
    ///
    /// A new `Logger` instance.
    pub fn new(log_file_path: &str, is_cpu: bool) -> Logger {
        let now = Local::now();
        // Format the date and time
        let formatted_time = now.format("%Y-%m-%d_%H-%M-%S").to_string();
        let name;
        if is_cpu {
            name = "cpu"
        } else {
            name = "backup"
        }
        // Create the log file name with date and time
        let log_file_name = format!("{}/{}_log_{}.txt", log_file_path, name, formatted_time);

        Logger {
            log_file_path: log_file_name,
        }
    }

    /// Logs the CPU usage to the log file.
    ///
    /// This function runs in a loop, logging the CPU usage every 2 minutes.
    pub fn log_cpu_usage(&self) {
        let mut system = System::new_all();
        let pid_num = std::process::id();
        let pid = Pid::from_u32(pid_num);

        system.refresh_all();
        thread::sleep(Duration::from_secs(1));
        system.refresh_all();

        loop {
            system.refresh_all();
            let cpu_usage = system.global_cpu_usage();
            let process = system.process(pid).expect("Process not found");
            let process_cpu_usage = process.cpu_usage();
            let num_cpus = system.cpus().len() as f32;
            println!("CORE: {}", num_cpus);
            let log_entry = format!(
                "Global CPU Usage: {:.2}%\t\tProcess CPU Usage: {:.2}%\n",
                cpu_usage, process_cpu_usage/num_cpus
            );
            /*let log_entry = format!("CPU Usage: {:.2}%\n", cpu_usage);*/
            self.write_log(&log_entry);
            thread::sleep(Duration::from_secs(1)); // Sleep for 2 minutes
        }
    }

    /// Logs the details of a completed backup to the log file.
    ///
    /// # Arguments
    ///
    /// * `total_size` - The total size of the backup in bytes.
    /// * `total_file` - The total number of files moved during the backup.
    /// * `cpu_time` - The duration of CPU time taken for the backup.
    pub fn log_backup_details(&self, total_size: u64, total_file: usize, cpu_time: Duration) {
        let readable_size = Self::bytes_to_human_readable(total_size);
        let log_entry = format!(
            "Backup completed. \n\nTotal size: \t\t{} ({} bytes) \nNumber of files: \t{} \nCPU time: \t\t{:.2?}\n",
            readable_size, total_size, total_file, cpu_time
        );
        self.write_log(&log_entry);
    }

    /// Converts a size in bytes to a human-readable string format.
    ///
    /// This function takes a size in bytes and converts it to a more readable format,
    /// such as KB, MB, or GB, depending on the size.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The size in bytes to be converted.
    ///
    /// # Returns
    ///
    /// A `String` representing the size in a human-readable format.
    fn bytes_to_human_readable(bytes: u64) -> String {
        const KIB: u64 = 1024;
        const MIB: u64 = 1024 * KIB;
        const GIB: u64 = 1024 * MIB;

        if bytes >= GIB {
            format!("{:.2} GB", bytes as f64 / GIB as f64)
        } else if bytes >= MIB {
            format!("{:.2} MB", bytes as f64 / MIB as f64)
        } else if bytes >= KIB {
            format!("{:.2} KB", bytes as f64 / KIB as f64)
        } else {
            format!("{} bytes", bytes) // Se meno di 1KB, mostra solo in byte
        }
    }

    /// Writes a log entry to the log file.
    ///
    /// # Arguments
    ///
    /// * `log_entry` - The log entry to be written to the file.
    pub fn write_log(&self, log_entry: &str) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
            .expect("Unable to open log file");

        file.write_all(log_entry.as_bytes())
            .expect("Unable to write to log file");
    }
}

#[cfg(not(debug_assertions))]
pub fn retrieve_path_cpu_log() -> PathBuf {
    use std::env;

    let exe_path = env::current_exe().expect("Failed to get current executable path");
    let exe_dir = exe_path.parent().expect("Failed to get executable directory");

    exe_dir.join("log/")
}

#[cfg(debug_assertions)]
pub fn retrieve_path_cpu_log() -> PathBuf {
    PathBuf::from("log/")
}