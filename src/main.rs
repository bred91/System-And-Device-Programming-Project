//#![windows_subsystem = "windows"]
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::backup::wrapper_backup;
use crate::config::Config;
use crate::logger::Logger;
use group_39::buttons_and_clicks_pattern_recognizer::start_button_and_clicks_pattern_recognizer;
use group_39::notification_popup::{show_popup, NotificationType};
use pattern_recognizer::PatternRecognizer;
use tokio::runtime;
mod notification_popup;
mod pattern_recognizer;
mod logger;
mod beeper;
mod backup;
mod config;

fn main() {
    let cpu_log_path = logger::retrieve_path_cpu_log().clone();
    let cpu_logger = Logger::new(cpu_log_path.to_str().unwrap(), true);
    let mut total_files = 0;
    let mut total_size = 0u64;

    // Start logging CPU usage in a separate thread <- no needs to wait
    let cpu_logger_clone = cpu_logger.clone();
    thread::spawn(move || {
        cpu_logger_clone.log_cpu_usage();
    });

    let config = Config::retrieve_and_check_config_file();
    //println!("Configuration loaded: {:?}", config);

    if config.btn_rec {
        start_button_and_clicks_pattern_recognizer();
    } else {
        let mut pat_pat = PatternRecognizer::new();
        pat_pat.recognize_pattern();
    }

    // Start of the backup operations
    let usb_logger = Logger::new(config.path_dest_backup.to_str().unwrap(), false);
    let start_time = Instant::now();

    cpu_logger.write_log("Inizia Backup\n");
    // backup
    let rt = runtime::Runtime::new().unwrap();
    rt.block_on(wrapper_backup(config, &mut total_files, &mut total_size)).unwrap();

    let cpu_time = start_time.elapsed();
    cpu_logger.write_log("Finisce Backup\n");
    // Emit a beep sound in a separate thread and get the handle
    let beep_thread = beeper::emit_beep(true);

    // Log backup details
    usb_logger.log_backup_details(total_size, total_files, cpu_time);

    // Wait for the beep threads to finish
    beep_thread.join().expect("Beep thread panicked");

    show_popup(NotificationType::BackupDone, None);
    thread::sleep(Duration::from_secs(10));
}