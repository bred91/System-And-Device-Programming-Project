extern crate libc;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use async_recursion::async_recursion;
use crate::config::Config;
use crate::notification_popup::{show_popup, NotificationType};
use tokio::fs::{self, File};
use tokio::io::{self, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

#[cfg(target_os = "windows")]
pub fn get_max_open_files() -> usize {
	// Numero approssimativo ragionevole per Windows
	// Questo valore può essere regolato in base alle tue necessità e osservazioni
	8192
}

#[cfg(not(target_os = "windows"))]
pub fn get_max_open_files() -> usize {
	let mut limits = libc::rlimit {
		rlim_cur: 0,
		rlim_max: 0
	};

	unsafe {
		libc::getrlimit(libc::RLIMIT_NOFILE, &mut limits);
	}

	550
}


/// Calculates the total number of files and their cumulative size within a given path,
/// recursively considering only the files of specified types as defined in config.yaml, if set; otherwise, all files are considered.
///
/// # Arguments
///
/// * `source` - A reference to the path to scan.
/// * `type_files` - A vector of strings representing the file types to include in the count.
///
/// # Returns
///
/// * An `io::Result` containing a tuple of the total file count and cumulative file size, or an error if the operation fails.
#[async_recursion]
pub async fn calculate_total_files(source: &Path, type_files: &Vec<String>) -> io::Result<(usize, u64)> {
	let mut count = 0;
	let mut total_size = 0u64;

	if source.is_dir() {
		let mut entries = fs::read_dir(source).await?;
		while let Some(entry) = entries.next_entry().await? {
			let path = entry.path();
			if path.is_dir() {
				//Box::pin is used to prevent asynchronous functions from moving in the heap during recursive operations.
				let (inner_count, inner_size) = Box::pin(calculate_total_files(&path, type_files)).await?;
				count += inner_count;
				total_size += inner_size;
			} else {
				if type_files.is_empty() || is_file_type_accepted(&path, type_files) {
					count += 1;
					total_size += fs::metadata(&path).await?.len();
				}
			}
		}
	}

	Ok((count, total_size))
}

/// Schedules backup tasks for each file and directory within a given source directory.
/// It recursively identifies all files and directories to be backed up and adds them to a task list.
///
/// # Arguments
///
/// * `source` - A reference to the path of the directory where files are sourced.
/// * `destination` - A reference to the path where files will be backed up.
/// * `type_files` - A vector of strings representing the file types to include in the backup.
/// * `tasks` - A mutable reference to a vector that will store the paths of source files and their corresponding backup destinations.
///
/// # Returns
///
/// * An `io::Result<()>` indicating success or failure of the task scheduling.
#[async_recursion]
async fn schedule_backup_tasks(source: &Path, destination: &Path, type_files: &Vec<String>, tasks: &mut Vec<(PathBuf, PathBuf)>) -> io::Result<()> {
	if source.is_dir() {
		fs::create_dir_all(destination).await?;

		let mut entries = fs::read_dir(source).await?;
		while let Some(entry) = entries.next_entry().await? {
			let path = entry.path();
			let new_destination = destination.join(path.file_name().unwrap());
			if path.is_dir() {
				Box::pin(schedule_backup_tasks(&path, &new_destination, type_files, tasks)).await?;
			} else {
				tasks.push((path, new_destination));
			}
		}
	}
	Ok(())
}

/// Executes the backup operation for files specified in the task list using asynchronous file operations.
/// This function also manages file access limits and progress reporting if enabled.
///
/// # Arguments
///
/// * `source` - A reference to the source directory path.
/// * `destination` - A reference to the destination directory path.
/// * `type_files` - A vector of strings detailing which file types should be backed up.
/// * `verbose` - A boolean flag to enable verbose progress output.
/// * `total_files` - The total number of files expected to be processed for backup.
/// * `copied_files` - An atomic reference to the count of files successfully copied.
/// * `last_printed_percent` - An atomic reference to the last printed percentage of progress.
/// * `max_file_opened` - The maximum number of file handles that can be opened concurrently during the backup.
///
/// # Returns
///
/// * An `io::Result<()>` indicating the success or failure of the backup operation.
pub async fn backup(source: &Path, destination: &Path, type_files: &Vec<String>, verbose: bool, total_files: usize, copied_files: Arc<Mutex<usize>>, last_printed_percent: Arc<Mutex<usize>>, max_file_opened: usize) -> io::Result<()> {
	let mut tasks: Vec<(PathBuf, PathBuf)> = Vec::new();

	// Fill the task list by calling a recursive function to identify files and directories for backup. Each with original path and destination path.
	schedule_backup_tasks(source, destination, type_files, &mut tasks).await?;

	// Create a semaphore to limit concurrent file operations to the maximum allowed.
	let semaphore = Arc::new(Semaphore::new(max_file_opened));

	// Initialize a vector to store asynchronous file copy threads.
	let mut handles: Vec<JoinHandle<()>> = vec![];

	for (path, dest_path) in tasks {
		if type_files.is_empty() || is_file_type_accepted(&path, type_files) {
			// Clone semaphore to control the number of concurrent operations.
			let semaphore = semaphore.clone();
			// Acquire a permit to proceed with a file copy operation.
			let permit = semaphore.acquire_owned().await.unwrap();
			// Clone the atomic counters to update progress in each task.
			let copied_files_clone = copied_files.clone();
			let last_printed_percent_clone = last_printed_percent.clone();

			// Spawn an asynchronous task to copy each file.
			let handle = tokio::spawn(async move {
				if let Err(e) = copy_file(&path, &dest_path).await {
					println!("Failed to copy {:?}: {}", path, e);
				}
				drop(permit);
				// Lock the mutex to safely update the number of copied files.
				let mut copied = copied_files_clone.lock().unwrap();
				*copied += 1;
				if verbose {
					print_progress(*copied, total_files, &last_printed_percent_clone);
				}
			});
			handles.push(handle);
		}
	}

	// Await all the file copy tasks to complete.
	for handle in handles {
		let _ = handle.await;
	}

	Ok(())
}


/// Copies a file from a source path to a destination path using asynchronous I/O operations.
/// This function employs buffered reading and writing for efficient data transfer.
///
/// # Arguments
///
/// * `src` - A reference to the source file path.
/// * `dest` - A reference to the destination file path.
///
/// # Returns
///
/// * An `io::Result<()>` indicating the success or failure of the file copy operation.
pub async fn copy_file(src: &Path, dest: &Path) -> io::Result<()> {
	let mut reader = BufReader::new(File::open(src).await?);
	let mut writer = BufWriter::new(File::create(dest).await?);

	io::copy(&mut reader, &mut writer).await?;
	writer.flush().await?;
	Ok(())
}


/// Prints the current progress of a file copying operation as a percentage of total files copied.
///
/// # Arguments
///
/// * `copied_files` - The number of files that have been successfully copied so far.
/// * `total_files` - The total number of files that need to be copied.
/// * `last_printed_percent` - A reference to an atomic integer wrapped in a mutex that stores the last printed percentage, to avoid redundant messages.
fn print_progress(copied_files: usize, total_files: usize, last_printed_percent: &Arc<Mutex<usize>>) {
	let percent = copied_files * 100 / total_files;
	let mut last_percent = last_printed_percent.lock().unwrap();
	if percent > *last_percent {
		println!("Progress: {}% ({} of {} files)", percent, copied_files, total_files);
		*last_percent = percent;
	}
}


/// Determines if the file at the specified path has an extension that is included in the list of accepted file types.
/// This function checks the file extension against a list of specified types, returning true if it matches any of them.
///
/// # Arguments
///
/// * `path` - A reference to the file path whose type is to be checked.
/// * `type_files` - A vector of strings representing the file types to include (e.g., ".txt", ".jpg").
///
/// # Returns
///
/// * A boolean value indicating whether the file type is accepted based on its extension.
fn is_file_type_accepted(path: &Path, type_files: &Vec<String>) -> bool {
	path.extension()
		.and_then(|ext| ext.to_str())
		.map(|ext| format!(".{}", ext))
		.map(|ext| type_files.contains(&ext.to_string()))
		.unwrap_or(false)
}


/// Orchestrates the backup process by invoking necessary functions before to calculate file totals (calculate_total_files),
/// then execute the backup, and handle any errors or special conditions such as non-existent paths.
///
/// # Arguments
///
/// * `config` - A `Config` set by the configuration in the config.yaml file
/// * `final_total_files` - A mutable reference to the main counter for the total number of files.
/// * `final_total_size` - A mutable reference to the main counter for the total size of the files.
///
/// # Returns
///
/// * A `Result<(), Box<dyn std::error::Error>>` indicating the success or failure of the backup operation.
pub async fn wrapper_backup(config: Config, final_total_files: &mut usize, final_total_size: &mut u64) -> Result<(), Box<dyn std::error::Error>> {
	if config.path_orig_backup.exists() && config.path_dest_backup.exists() {
		let (total_files, total_size) = calculate_total_files(config.path_orig_backup.as_path(), &config.type_files).await?;
		*final_total_size = total_size;
		*final_total_files = total_files;
		let copied_files = Arc::new(Mutex::new(0));
		let last_printed_percent = Arc::new(Mutex::new(0));
		let max_file_opened = get_max_open_files();
		if total_files > 0 {
			backup(config.path_orig_backup.as_path(), config.path_dest_backup.as_path(), &config.type_files, true, total_files, copied_files.clone(), last_printed_percent.clone(), max_file_opened).await?;
			Ok(())
		} else {
			show_popup(NotificationType::GenericError, Some("No files to copy.".to_string()));
			Ok(())
		}
	} else {
		if !config.path_orig_backup.exists() {
			show_popup(NotificationType::GenericError, Some(format!("Error: Source path does not exist: {:?}", config.path_orig_backup)));
			return Ok(())
		}
		if !config.path_dest_backup.exists() {
			show_popup(NotificationType::GenericError, Some(format!("Error: Destination path does not exist: {:?}", config.path_dest_backup)));
			return Ok(())
		}
		Ok(())
	}

}