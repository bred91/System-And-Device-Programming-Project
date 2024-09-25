use crate::notification_popup::{show_popup, NotificationType};
use notify::{Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
	pub path_dest_backup: PathBuf,
	pub path_orig_backup: PathBuf,
	pub type_files: Vec<String>,
	pub btn_rec: bool
}
impl Config {
	/// Reads the configuration from a file.
	///
	/// # Arguments
	///
	/// * `path` - A reference to a path that points to the configuration file.
	///
	/// # Returns
	///
	/// A `Result` containing the `Config` object if the file is read successfully, or a boxed `Error` if an error occurs.
	///
	/// # Errors
	///
	/// This function will return an error if the file cannot be opened, read, or if the contents cannot be parsed as YAML.

	pub fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
		let mut file = File::open(path)?;
		let mut contents = String::new();
		file.read_to_string(&mut contents)?;
		let mut config:Config = serde_yaml::from_str(&contents)?;

		if !config.type_files.is_empty() {
			config.type_files = config.type_files.iter().map(|f| {
				if !f.starts_with('.') {
					format!(".{}", f)
				} else {
					f.clone()
				}
			}).collect();
		}
		Ok(config)
	}


	/// Reads and checks the configuration file, and sets up a file watcher to monitor changes.
	///
	/// # Returns
	///
	/// * `Config` - The configuration object read from the file.
	pub fn retrieve_and_check_config_file() -> Config {
		let binding = Self::retrieve_path_config().clone();
  		let path_config: &str = binding.to_str().unwrap();

		// Initial attempt to read the configuration file
		match Config::read_from_file(path_config) {
			// if ok, then it returns the config
			Ok(config) => return config,
			Err(e) => Self::handle_config_error(&e.to_string()),
		}

		// otherwise, we need to watch for a modification (correction) of the file
		let mut last_event: HashMap<String, Instant> = HashMap::new();
		let debounce_duration = Duration::from_millis(500);
		let (tx, rx) = channel();

		// Create a watcher object, delivering debounced events.
		let notify_config = NotifyConfig::default().with_poll_interval(Duration::from_secs(2));
		let mut watcher: RecommendedWatcher = Watcher::new(tx.clone(), notify_config).unwrap();

		// Add a path to be watched. All files and directories at that path and below will be monitored for changes.
		watcher.watch(path_config.as_ref(), RecursiveMode::NonRecursive).unwrap();

		while let Ok(event) = rx.recv() {
			if let Ok(event) = event {
				let path = event.paths[0].to_str().unwrap().to_string();
				let now = Instant::now();

				// Check if the event is too close to the last one and ignore it if so
				// this is because we can have a debounce
				// for example: many text editors, when saving a file, perform two operations:
				// they write the contents into a temporary file and then rename the temporary file
				// with the name of the original file.
				// This can generate two edit events (Modify).
				if let Some(last_time) = last_event.get(&path) {
					if now.duration_since(*last_time) < debounce_duration {
						continue; // Ignore this event as it is too close to the last one
					}
				}

				// Update the last event time for the path
				last_event.insert(path.clone(), now);

				// If the event was a modify one, I can read again the file to check that everything is ok
				match event.kind {
					EventKind::Modify(_) => {
						match Config::read_from_file(path_config) {
							Ok(config) => {
								drop(tx); // if ok, drop the sender to stop the watcher
								return config;
							}
							Err(e) => Self::handle_config_error(&e.to_string()),
						}
					}
					_ => {}
				}
			}
		}

		panic!("Failed to read initial configuration");
	}

	#[cfg(not(debug_assertions))]
	fn retrieve_path_config() -> PathBuf {
		use std::env;

		let exe_path = env::current_exe().expect("Failed to get current executable path");
		let exe_dir = exe_path.parent().expect("Failed to get executable directory");

		exe_dir.join("config.yaml")
	}

	#[cfg(debug_assertions)]
	fn retrieve_path_config() -> PathBuf {
		PathBuf::from("config.yaml")
	}

	/// Handles configuration errors by displaying the appropriate notifications.
	///
	/// # Arguments
	///
	/// * `error_message` - A string slice that holds the error message.
	fn handle_config_error(error_message: &str) {
		// Check if the error is due to a missing field in the configuration file
		if error_message.contains("missing field") {
			// Show a configuration error popup with the missing field information
			show_popup(NotificationType::ConfigError,
					   Some(format!(
						   " {} non presente nel file di configurazione",
						   error_message.split('`').nth(1).unwrap_or("unknown field").to_string(),
					   ))
			);
		} else {
			// Show a generic error popup with the error message
			show_popup(NotificationType::GenericError, Some(error_message.to_string()));
		}
	}
}