use std::ffi::OsStr;
use std::sync::{Mutex, Arc};
use std::thread::{self, sleep};
use std::time::Duration;
use std::{collections::{HashMap, HashSet}, ops::Deref};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::fs::{self, OpenOptions, File};
use std::io::{Write, Cursor, copy, BufReader};
use zip::ZipArchive;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::{cloud_settings::CloudSettings, update_descriptor::UpdateDescriptor};

pub struct UpdateStore {
    _settings: CloudSettings,
    store_root_folder: PathBuf,
    store_directory: PathBuf,
    updates: HashMap<String, Arc<Mutex<UpdateDescriptor>>>,
    jwt: String
}

impl UpdateStore {
    const UPDATE_INFO_FILE_NAME: &'static str = "info.json";

    pub fn new(settings: CloudSettings) -> UpdateStore {
        let store_root = settings.update_store_path.clone();

        let mut store = UpdateStore {
            _settings : settings,
            store_root_folder: store_root.clone(),
            store_directory: store_root,
            updates: HashMap::new(),
            jwt: String::new()
        };
        
        println!("Update data will be stored in '{:?}'", store.store_directory);

        if ! store.store_directory.exists() {
            if let Err(e) = fs::create_dir_all(&store.store_directory) {
                eprintln!("WARNING: Failed to create store directory: {}. Store may not function properly.", e);
            }
        }
        else {
            // load all existing update descriptors
            match fs::read_dir(&store.store_directory) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(e) => {
                                if e.path().is_dir() {
                                    // it's a likely update folder, but look for (and parse) an info file to be sure
                                    match fs::read_dir(e.path()) {
                                        Ok(sub_entries) => {
                                            for entry in sub_entries {
                                match entry {
                                    Ok(f) => {
                                        let fp = f.path();
                                        let file_name = fp.file_name().unwrap_or(OsStr::new(""));
                                        if fp.is_file() && file_name == Self::UPDATE_INFO_FILE_NAME {
                                            println!("Update found: {:?}", e.file_name());

                                            match File::open(fp) {
                                                Ok(file) => {
                                                    let reader = BufReader::new(file);
                                                    match serde_json::from_reader(reader) {
                                                        Ok(descriptor) => {
                                                            // TODO: verify the mpak existence for "retrieved" items?
                                                            store.add(Arc::new(descriptor))
                                                        },
                                                        Err(err) => {
                                                            println!("Cannot deserialize info for {:?}: {:?}", e.file_name(), err);
                                                        }        
                                                    }
                                                },
                                                Err(err) => {
                                                    println!("Cannot open info file for {:?}: {:?}", e.file_name(), err);
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("WARNING: Failed to read entry in update folder: {}", e);
                                    }
                                }
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("WARNING: Failed to read update subfolder: {}", e);
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                eprintln!("WARNING: Failed to read store entry: {}", e);
                            }
                        }
                    }
                },
                Err(e) => {
                    eprintln!("ERROR: Failed to read store directory: {}", e);
                }
            }
        }

        store
    }

    pub fn get_all_messages(&self) -> Vec<Arc<Mutex<UpdateDescriptor>>> {
        self.updates.values().cloned().collect::<Vec<Arc<Mutex<UpdateDescriptor>>>>()        
    }

    pub fn add(&mut self, descriptor: Arc<UpdateDescriptor>) {
        let rf = Arc::new( Mutex::new((*descriptor).clone()));
        let id = descriptor.deref().mpak_id.clone();
        self.updates.insert(id, rf);
        self.save_or_update(descriptor.deref());
    }

    pub fn len(&self) -> i32 {
        self.updates.len() as i32
    }

    pub fn get_message(&self, id: String) -> Option<&Arc<Mutex<UpdateDescriptor>>> {
        self.updates.get(&id)
    }

    pub fn remove_update(&mut self, mpak_id: String) {
        match fs::read_dir(self.store_directory.clone()) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(e) => {
                            match e.file_name().into_string() {
                                Ok(name) if name == mpak_id => {
                                    if e.path().is_dir() {
                                        // it's a likely update folder, but look for (and parse) an info file to be sure
                                        match fs::read_dir(e.path()) {
                                            Ok(sub_entries) => {
                                                for entry in sub_entries {
                                match entry {
                                    Ok(f) => {
                                        let fp = f.path();
                                        let file_name = fp.file_name().unwrap_or(OsStr::new(""));
                                        if fp.is_file() && file_name == Self::UPDATE_INFO_FILE_NAME {
                                            if let Err(e) = fs::remove_dir_all(e.path()) {
                                                eprintln!("ERROR: Failed to remove update directory: {}", e);
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("WARNING: Failed to read entry: {}", e);
                                    }
                                }
                                                            }
                                                        },
                                                        Err(e) => {
                                                            eprintln!("WARNING: Failed to read update folder: {}", e);
                                                        }
                                                    }
                                                }
                                                self.updates.remove(&mpak_id);
                                                return;
                                            },
                                            _ => {} // Name doesn't match or conversion failed
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("WARNING: Failed to read directory entry: {}", e);
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("ERROR: Failed to read store directory for removal: {}", e);
                        }
                    }
    }

    pub fn clear(&mut self) {
        let id_list: Vec<String> = self.updates.keys().cloned().collect();
        for id in id_list {
            self.remove_update(id);
        }

        self.updates.clear();
    }

    /// Get the application directory from the application path
    ///
    /// Examples:
    /// - /home/user/myapp/myapp -> /home/user/myapp
    /// - /opt/myapp/bin/myapp -> /opt/myapp/bin
    fn get_app_directory(app_path: &PathBuf) -> Result<PathBuf, String> {
        let app_folder = app_path.parent()
            .ok_or_else(|| "Failed to get application folder from path".to_string())?;
        Ok(app_folder.to_path_buf())
    }

    /// Collect all files in a package directory recursively
    /// Returns a HashSet of relative paths for quick lookup
    fn collect_package_files(package_dir: &Path) -> Result<HashSet<PathBuf>, String> {
        let mut files = HashSet::new();

        if !package_dir.exists() {
            return Err(format!("Package directory does not exist: {:?}", package_dir));
        }

        Self::collect_files_recursive(package_dir, package_dir, &mut files)?;

        Ok(files)
    }

    /// Recursive helper for collect_package_files
    fn collect_files_recursive(base_dir: &Path, current_dir: &Path, files: &mut HashSet<PathBuf>) -> Result<(), String> {
        match fs::read_dir(current_dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(e) => {
                            let path = e.path();
                            if path.is_file() {
                                // Store relative path from base_dir
                                if let Ok(rel_path) = path.strip_prefix(base_dir) {
                                    files.insert(rel_path.to_path_buf());
                                }
                            } else if path.is_dir() {
                                Self::collect_files_recursive(base_dir, &path, files)?;
                            }
                        }
                        Err(e) => {
                            eprintln!("WARNING: Failed to read directory entry: {}", e);
                        }
                    }
                }
                Ok(())
            }
            Err(e) => Err(format!("Failed to read directory {:?}: {}", current_dir, e))
        }
    }

    /// Merge preserved files from source directory to destination
    /// Copies any file from source that doesn't exist in new_files set
    fn merge_preserved_files(source_dir: &Path, dest_dir: &Path, new_files: &HashSet<PathBuf>) -> Result<usize, String> {
        if !source_dir.exists() {
            return Err(format!("Source directory does not exist: {:?}", source_dir));
        }

        let preserved_count = Self::merge_files_recursive(source_dir, source_dir, dest_dir, new_files)?;

        Ok(preserved_count)
    }

    /// Recursive helper for merge_preserved_files
    fn merge_files_recursive(base_dir: &Path, current_dir: &Path, dest_base: &Path, new_files: &HashSet<PathBuf>) -> Result<usize, String> {
        let mut count = 0;

        match fs::read_dir(current_dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(e) => {
                            let path = e.path();
                            let rel_path = match path.strip_prefix(base_dir) {
                                Ok(p) => p,
                                Err(_) => continue,
                            };

                            if path.is_file() {
                                // Only copy if NOT in new_files set
                                if !new_files.contains(rel_path) {
                                    let dest_path = dest_base.join(rel_path);

                                    // Create parent directory if needed
                                    if let Some(parent) = dest_path.parent() {
                                        if let Err(e) = fs::create_dir_all(parent) {
                                            return Err(format!("Failed to create directory {:?}: {}", parent, e));
                                        }
                                    }

                                    // Copy the file
                                    if let Err(e) = fs::copy(&path, &dest_path) {
                                        return Err(format!("Failed to copy {:?} to {:?}: {}", path, dest_path, e));
                                    }

                                    count += 1;
                                }
                            } else if path.is_dir() {
                                count += Self::merge_files_recursive(base_dir, &path, dest_base, new_files)?;
                            }
                        }
                        Err(e) => {
                            eprintln!("WARNING: Failed to read directory entry: {}", e);
                        }
                    }
                }
                Ok(count)
            }
            Err(e) => Err(format!("Failed to read directory {:?}: {}", current_dir, e))
        }
    }

    /// Try atomic swap first, fallback to file-by-file if cross-device error
    ///
    /// This provides optimal performance when possible (atomic rename on same filesystem)
    /// while automatically falling back to robust file-by-file operations when needed
    fn swap_with_fallback(current: &Path, staging: &Path, rollback: &Path) -> Result<(), String> {
        println!("Attempting atomic directory swap...");

        match Self::atomic_directory_swap(current, staging, rollback) {
            Ok(_) => {
                println!("✓ Atomic swap succeeded");
                Ok(())
            }
            Err(e) => {
                // Check if it's a cross-device error
                if e.contains("cross-device") || e.contains("Invalid cross-device link") {
                    println!("⚠ Cross-device link detected, using file-by-file fallback");
                    Self::file_by_file_swap(current, staging, rollback)
                } else {
                    // Other error, propagate it
                    Err(e)
                }
            }
        }
    }

    /// Perform file-by-file swap (fallback for cross-filesystem scenarios)
    ///
    /// 1. Move current -> rollback (file by file)
    /// 2. Move staging -> current (file by file)
    ///
    /// This is slower than atomic rename but works across filesystems
    fn file_by_file_swap(current: &Path, staging: &Path, rollback: &Path) -> Result<(), String> {
        println!("FILE-BY-FILE SWAP: Using cross-filesystem fallback");

        // Clean up old rollback if it exists
        if rollback.exists() {
            println!("  Removing old rollback directory: {:?}", rollback);
            if let Err(e) = fs::remove_dir_all(rollback) {
                return Err(format!("Failed to remove old rollback directory: {}", e));
            }
        }

        // Create rollback directory
        if let Err(e) = fs::create_dir_all(rollback) {
            return Err(format!("Failed to create rollback directory: {}", e));
        }

        println!("OPERATION #1: Backing up current version (file-by-file)");
        println!("  Copy: {:?} -> {:?}", current, rollback);

        // Copy current version to rollback
        let opts = fs_extra::dir::CopyOptions::new()
            .overwrite(true)
            .content_only(true);

        if let Err(e) = fs_extra::dir::copy(current, rollback, &opts) {
            return Err(format!("Failed to backup current version to rollback: {}", e));
        }

        println!("OPERATION #2: Removing current version");
        // Remove all contents from current directory (but keep the directory itself)
        match fs::read_dir(current) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Err(e) = fs::remove_dir_all(&path) {
                                return Err(format!("Failed to remove directory {:?}: {}", path, e));
                            }
                        } else {
                            if let Err(e) = fs::remove_file(&path) {
                                return Err(format!("Failed to remove file {:?}: {}", path, e));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(format!("Failed to read current directory: {}", e));
            }
        }

        println!("OPERATION #3: Deploying new version (file-by-file)");
        println!("  Copy: {:?} -> {:?}", staging, current);

        // Copy staging to current
        if let Err(e) = fs_extra::dir::copy(staging, current, &opts) {
            // CRITICAL ERROR: Try to restore from rollback
            eprintln!("ERROR: Failed to deploy new version: {}", e);
            eprintln!("Attempting to restore from rollback...");

            if let Err(restore_err) = fs_extra::dir::copy(rollback, current, &opts) {
                return Err(format!(
                    "CRITICAL: Failed to deploy new version AND failed to restore from rollback! \
                    Original error: {}. Restore error: {}. \
                    Rollback is at: {:?}",
                    e, restore_err, rollback
                ));
            }

            return Err(format!("Failed to deploy new version (restored from rollback): {}", e));
        }

        println!("File-by-file swap completed successfully");
        println!("  New version active at: {:?}", current);
        println!("  Rollback available at: {:?}", rollback);

        Ok(())
    }

    /// Perform atomic directory swap using two rename operations
    ///
    /// 1. rename(current, rollback) - backup current version
    /// 2. rename(new, current) - activate new version
    ///
    /// If any operation fails, attempts to restore from rollback
    /// Returns Err with std::io::Error for cross-device detection
    fn atomic_directory_swap(current: &Path, new: &Path, rollback: &Path) -> Result<(), String> {
        println!("ATOMIC OPERATION #1: Backing up current version");
        println!("  Rename: {:?} -> {:?}", current, rollback);

        // Clean up old rollback if it exists
        if rollback.exists() {
            println!("  Removing old rollback directory: {:?}", rollback);
            if let Err(e) = fs::remove_dir_all(rollback) {
                return Err(format!("Failed to remove old rollback directory: {}", e));
            }
        }

        // ATOMIC OPERATION #1: Backup current version
        if let Err(e) = fs::rename(current, rollback) {
            return Err(format!("Failed to backup current version (rename {:?} -> {:?}): {}",
                current, rollback, e));
        }

        println!("ATOMIC OPERATION #2: Activating new version");
        println!("  Rename: {:?} -> {:?}", new, current);

        // ATOMIC OPERATION #2: Activate new version
        if let Err(e) = fs::rename(new, current) {
            // CRITICAL ERROR: Try to restore from rollback
            eprintln!("ERROR: Failed to activate new version: {}", e);
            eprintln!("Attempting to restore from rollback...");

            if let Err(restore_err) = fs::rename(rollback, current) {
                return Err(format!(
                    "CRITICAL: Failed to activate new version AND failed to restore rollback! \
                    Original error: {}. Restore error: {}. \
                    System may be in inconsistent state. Rollback is at: {:?}",
                    e, restore_err, rollback
                ));
            }

            return Err(format!("Failed to activate new version (restored from rollback): {}", e));
        }

        println!("Atomic directory swap completed successfully");
        println!("  New version active at: {:?}", current);
        println!("  Rollback available at: {:?}", rollback);

        Ok(())
    }

    pub async fn apply_update(&self, id: &String, app_path: &PathBuf, pid: i32, command: &Option<String>) -> Result<u64, String> {
        println!("APPLYING UPDATE {}", id);

        let p = app_path.clone();
        let update = match self.updates.get(id) {
            Some(u) => u.clone(),
            None => {
                let msg = format!("Update {} not found in store", id);
                eprintln!("ERROR: {}", msg);
                return Err(msg);
            }
        };

        // extract the update to a temp location
        let d = match update.lock() {
            Ok(descriptor) => descriptor,
            Err(e) => {
                let msg = format!("Failed to lock update descriptor: {}", e);
                eprintln!("ERROR: {}", msg);
                return Err(msg);
            }
        };
        let package_path = format!("{}/{}/update.mpak", self.store_root_folder.display(), d.mpak_id);
        let update_temp_path = &self._settings.temp_extract_path;

        // Clean temp folder before extracting
        if update_temp_path.exists() {
            println!("Cleaning temp extract folder: {:?}", update_temp_path);
            if let Err(e) = fs::remove_dir_all(update_temp_path) {
                eprintln!("WARNING: Failed to clean temp extract folder: {}", e);
            }
        }
        // Recreate the directory
        if let Err(e) = fs::create_dir_all(update_temp_path) {
            let msg = format!("Failed to create temp extract directory: {}", e);
            eprintln!("ERROR: {}", msg);
            return Err(msg);
        }

        let update_temp_path_str = update_temp_path.to_string_lossy().to_string();
        if let Err(e) = self.extract_package_to_location(package_path, &update_temp_path_str) {
            let msg = format!("Failed to extract package: {}", e);
            eprintln!("ERROR: {}", msg);
            return Err(msg);
        }

        // make sure it's a valid app update (i.e. has an `app` folder)
        let update_source_folder = update_temp_path.join("app");
        if !update_source_folder.is_dir() {
            println!("Not a valid app update");
            return Err("Package does not contain a valid Application update".to_string());
        }

        // spawn a thread to wait for app shutdown
        let local_command = command.clone();
        let timeout_seconds = self._settings.update_apply_timeout_seconds;
        let temp_path = update_temp_path.clone();
        let update_id = id.clone();
        let store_root = self.store_root_folder.clone();
        let settings = self._settings.clone();

        thread::spawn(move || {
            let application_folder = match p.parent().and_then(|p| p.to_str()) {
                Some(folder) => folder,
                None => {
                    eprintln!("ERROR: Failed to get application folder from path");
                    return;
                }
            };
            let app = match p.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => {
                    eprintln!("ERROR: Failed to get application name from path");
                    return;
                }
            };
            let proc_folder = format!("/proc/{}", pid);
            let proc_path = Path::new(&proc_folder);

            println!("Caller is '{}' (PID {}) running from '{}'", app, pid, application_folder);

            // If app is managed by systemd, stop the service to prevent auto-restart
            if settings.app_is_systemd_service {
                if let Some(ref service_name) = settings.app_service_name {
                    println!("Stopping systemd service '{}'...", service_name);
                    match Command::new("systemctl")
                        .arg("stop")
                        .arg(service_name)
                        .output() {
                        Ok(output) => {
                            if output.status.success() {
                                println!("Successfully stopped service '{}'", service_name);
                            } else {
                                eprintln!("WARNING: Failed to stop service '{}': {}",
                                    service_name,
                                    String::from_utf8_lossy(&output.stderr));
                            }
                        },
                        Err(e) => {
                            eprintln!("ERROR: Failed to execute systemctl stop: {}", e);
                            eprintln!("Update may fail if systemd auto-restarts the service");
                        }
                    }
                } else {
                    eprintln!("ERROR: app_is_systemd_service is true but app_service_name is not set!");
                    eprintln!("Update may fail if systemd auto-restarts the service");
                }
            }

            println!("Waiting for process to exit (timeout: {} seconds)", timeout_seconds);

            let start_time = std::time::Instant::now();
            let mut last_warning = 0u64;

            loop {
                // dev note: there's probably a better way to do this, but I can't find it
                // wait::waitpid only works for child processes

                let elapsed_secs = start_time.elapsed().as_secs();

                // Check for timeout
                if elapsed_secs >= timeout_seconds {
                    println!("ERROR: Timeout waiting for '{}' to exit after {} seconds", app, timeout_seconds);
                    println!("Cleaning up temp extraction folder: {}", temp_path.display());
                    let _ = fs::remove_dir_all(&temp_path);
                    // TODO: Mark update as "failed" in descriptor
                    return;
                }

                // Log warnings at milestone intervals (1 min, 2 min, 3 min, 4 min)
                let current_minute = elapsed_secs / 60;
                if current_minute > last_warning && current_minute > 0 {
                    println!("WARNING: Still waiting for '{}' to exit ({} minutes elapsed)", app, current_minute);
                    last_warning = current_minute;
                }

                match proc_path.is_dir() {
                    true => {
                        sleep(Duration::from_millis(1000));
                    },
                    _ => {
                        println!("'{}' exited after {} seconds", &app, start_time.elapsed().as_secs());

                        // Get application directory
                        let app_dir = match Self::get_app_directory(&p) {
                            Ok(dir) => dir,
                            Err(e) => {
                                eprintln!("ERROR: Failed to get application directory: {}", e);
                                eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                                let _ = fs::remove_dir_all(&temp_path);
                                return;
                            }
                        };

                        println!("Application directory: {:?}", app_dir);

                        // Use temp staging from settings as working area
                        let temp_staging_dir = settings.staging_path.clone();

                        println!("Temp staging directory: {:?}", temp_staging_dir);

                        // Clean up any existing temp staging directory
                        if temp_staging_dir.exists() {
                            println!("Removing existing temp staging directory: {:?}", temp_staging_dir);
                            if let Err(e) = fs::remove_dir_all(&temp_staging_dir) {
                                eprintln!("ERROR: Failed to remove existing temp staging directory: {}", e);
                                eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                                let _ = fs::remove_dir_all(&temp_path);
                                return;
                            }
                        }

                        // Create temp staging directory
                        if let Err(e) = fs::create_dir_all(&temp_staging_dir) {
                            eprintln!("ERROR: Failed to create temp staging directory: {}", e);
                            eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                            let _ = fs::remove_dir_all(&temp_path);
                            return;
                        }

                        // Copy new files from extracted package to temp staging directory
                        println!("Copying new files from package to {:?}", temp_staging_dir);
                        let opts = fs_extra::dir::CopyOptions::new()
                            .overwrite(true)
                            .content_only(true);

                        if let Err(e) = fs_extra::dir::copy(&update_source_folder, &temp_staging_dir, &opts) {
                            eprintln!("ERROR: Failed to copy new files: {}", e);
                            eprintln!("Cleaning up temp staging directory: {:?}", temp_staging_dir);
                            let _ = fs::remove_dir_all(&temp_staging_dir);
                            eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                            let _ = fs::remove_dir_all(&temp_path);
                            return;
                        }

                        // Collect list of files in the package (for preservation logic)
                        let new_files = match Self::collect_package_files(&update_source_folder) {
                            Ok(files) => files,
                            Err(e) => {
                                eprintln!("ERROR: Failed to collect package files: {}", e);
                                eprintln!("Cleaning up temp staging directory: {:?}", temp_staging_dir);
                                let _ = fs::remove_dir_all(&temp_staging_dir);
                                eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                                let _ = fs::remove_dir_all(&temp_path);
                                return;
                            }
                        };

                        println!("Package contains {} files", new_files.len());

                        // Merge preserved files from current version
                        println!("Merging preserved files from current version...");
                        match Self::merge_preserved_files(&app_dir, &temp_staging_dir, &new_files) {
                            Ok(count) => {
                                println!("Preserved {} files from current version", count);
                            }
                            Err(e) => {
                                eprintln!("ERROR: Failed to merge preserved files: {}", e);
                                eprintln!("Cleaning up temp staging directory: {:?}", temp_staging_dir);
                                let _ = fs::remove_dir_all(&temp_staging_dir);
                                eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                                let _ = fs::remove_dir_all(&temp_path);
                                return;
                            }
                        }

                        // Get rollback directory from settings (in temp)
                        let rollback_dir = settings.rollback_path.clone();
                        println!("Staging directory: {:?}", temp_staging_dir);
                        println!("Rollback directory: {:?}", rollback_dir);

                        // Perform directory swap (will use file-by-file for cross-filesystem)
                        if let Err(e) = Self::swap_with_fallback(&app_dir, &temp_staging_dir, &rollback_dir) {
                            eprintln!("ERROR: Directory swap failed: {}", e);
                            eprintln!("Cleaning up temp staging directory: {:?}", temp_staging_dir);
                            let _ = fs::remove_dir_all(&temp_staging_dir);
                            eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                            let _ = fs::remove_dir_all(&temp_path);
                            return;
                        }

                        // Mark update as "applied" in descriptor
                        Self::mark_update_applied(&update_id, &store_root);

                        // Clean up temp staging directory
                        println!("Cleaning up temp staging directory: {:?}", temp_staging_dir);
                        let _ = fs::remove_dir_all(&temp_staging_dir);

                        // Clean up temp extraction folder
                        println!("Cleaning up temp extraction folder: {}", temp_path.display());
                        let _ = fs::remove_dir_all(&temp_path);

                        // Update completed successfully
                        println!("Update applied successfully!");
                        println!("  Active version: {:?}", app_dir);
                        println!("  Rollback available: {:?}", rollback_dir);

                        // Restart the app
                        if settings.app_is_systemd_service {
                            // Restart via systemd
                            if let Some(ref service_name) = settings.app_service_name {
                                println!("Starting systemd service '{}'...", service_name);
                                match Command::new("systemctl")
                                    .arg("start")
                                    .arg(service_name)
                                    .output() {
                                    Ok(output) => {
                                        if output.status.success() {
                                            println!("Successfully started service '{}'", service_name);
                                        } else {
                                            eprintln!("ERROR: Failed to start service '{}': {}",
                                                service_name,
                                                String::from_utf8_lossy(&output.stderr));
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("ERROR: Failed to execute systemctl start: {}", e);
                                    }
                                }
                            } else {
                                eprintln!("ERROR: app_is_systemd_service is true but app_service_name is not set!");
                            }
                        } else {
                            // Direct process spawn
                            println!("Launching '{:?}' in directory '{}'...", p, application_folder);
                            match local_command {
                                None => {
                                    let _app = Command::new(&p)
                                        .current_dir(application_folder)
                                        .stdin(Stdio::null())
                                        .stdout(Stdio::null())
                                        .stderr(Stdio::null())
                                        .process_group(0)
                                        .spawn()
                                        .expect("Failed to start process");
                                },
                                Some(cmd) => {
                                    let _app = Command::new(cmd)
                                        .arg(&p)
                                        .current_dir(application_folder)
                                        .stdin(Stdio::null())
                                        .stdout(Stdio::null())
                                        .stderr(Stdio::null())
                                        .process_group(0)
                                        .spawn()
                                        .expect("Failed to start process");
                                },
                            }
                        }

                        return;
                    }
                }
            }
        });

        Ok(1)
    }

    /// Apply an already-extracted update without tracking
    ///
    /// This method is for external applications that handle extraction themselves.
    /// It assumes the update has already been extracted to temp_extract_path/app.
    ///
    /// # Arguments
    /// * `app_dir` - Application directory where update will be applied
    /// * `executable_path` - Full path to executable/DLL for restart (can be relative to app_dir)
    /// * `pid` - Process ID to wait for before applying
    /// * `command` - Optional command to use for restart (e.g., "dotnet" for .NET apps)
    pub async fn apply_extracted_update(&self, app_dir: &PathBuf, executable_path: &PathBuf, pid: i32, command: &Option<String>) -> Result<u64, String> {
        println!("APPLYING EXTRACTED UPDATE (no tracking)");

        // Verify the extracted update exists
        let update_temp_path = &self._settings.temp_extract_path;
        let update_source_folder = update_temp_path.join("app");

        if !update_source_folder.exists() {
            let msg = format!("Extracted update not found at {:?}", update_source_folder);
            eprintln!("ERROR: {}", msg);
            return Err(msg);
        }

        if !update_source_folder.is_dir() {
            let msg = format!("Update source is not a directory: {:?}", update_source_folder);
            eprintln!("ERROR: {}", msg);
            return Err(msg);
        }

        println!("Update source folder: {:?}", update_source_folder);

        // Clone for thread
        let app_dir_clone = app_dir.clone();
        let executable_path_clone = executable_path.clone();
        let local_command = command.clone();
        let timeout_seconds = self._settings.update_apply_timeout_seconds;
        let temp_path = update_temp_path.clone();
        let settings = self._settings.clone();

        thread::spawn(move || {
            let app_dir_str = app_dir_clone.to_string_lossy().to_string();
            let executable_name = executable_path_clone.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            let proc_folder = format!("/proc/{}", pid);
            let proc_path = Path::new(&proc_folder);

            println!("Caller is '{}' (PID {}) running from '{}'", executable_name, pid, app_dir_str);

            // If app is managed by systemd, stop the service to prevent auto-restart
            if settings.app_is_systemd_service {
                if let Some(ref service_name) = settings.app_service_name {
                    println!("Stopping systemd service '{}'...", service_name);
                    match Command::new("systemctl")
                        .arg("stop")
                        .arg(service_name)
                        .output() {
                        Ok(output) => {
                            if output.status.success() {
                                println!("Successfully stopped service '{}'", service_name);
                            } else {
                                eprintln!("WARNING: Failed to stop service '{}': {}",
                                    service_name,
                                    String::from_utf8_lossy(&output.stderr));
                            }
                        },
                        Err(e) => {
                            eprintln!("ERROR: Failed to execute systemctl stop: {}", e);
                            eprintln!("Update may fail if systemd auto-restarts the service");
                        }
                    }
                } else {
                    eprintln!("ERROR: app_is_systemd_service is true but app_service_name is not set!");
                    eprintln!("Update may fail if systemd auto-restarts the service");
                }
            }

            println!("Waiting for process to exit (timeout: {} seconds)", timeout_seconds);

            let start_time = std::time::Instant::now();
            let mut last_warning = 0u64;

            loop {
                let elapsed_secs = start_time.elapsed().as_secs();

                // Check for timeout
                if elapsed_secs >= timeout_seconds {
                    println!("ERROR: Timeout waiting for '{}' to exit after {} seconds", executable_name, timeout_seconds);
                    println!("Cleaning up temp extraction folder: {}", temp_path.display());
                    let _ = fs::remove_dir_all(&temp_path);
                    return;
                }

                // Log warnings at milestone intervals (1 min, 2 min, 3 min, 4 min)
                let current_minute = elapsed_secs / 60;
                if current_minute > last_warning && current_minute > 0 {
                    println!("WARNING: Still waiting for '{}' to exit ({} minutes elapsed)", executable_name, current_minute);
                    last_warning = current_minute;
                }

                match proc_path.is_dir() {
                    true => {
                        sleep(Duration::from_millis(1000));
                    },
                    _ => {
                        println!("'{}' exited after {} seconds", executable_name, start_time.elapsed().as_secs());

                        // Use the app_dir that was provided
                        println!("Application directory: {:?}", app_dir_clone);

                        // Use temp staging from settings as working area
                        let temp_staging_dir = settings.staging_path.clone();

                        println!("Temp staging directory: {:?}", temp_staging_dir);

                        // Clean up any existing temp staging directory
                        if temp_staging_dir.exists() {
                            println!("Removing existing temp staging directory: {:?}", temp_staging_dir);
                            if let Err(e) = fs::remove_dir_all(&temp_staging_dir) {
                                eprintln!("ERROR: Failed to remove existing temp staging directory: {}", e);
                                eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                                let _ = fs::remove_dir_all(&temp_path);
                                return;
                            }
                        }

                        // Create temp staging directory
                        if let Err(e) = fs::create_dir_all(&temp_staging_dir) {
                            eprintln!("ERROR: Failed to create temp staging directory: {}", e);
                            eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                            let _ = fs::remove_dir_all(&temp_path);
                            return;
                        }

                        // Copy new files from extracted package to temp staging directory
                        println!("Copying new files from package to {:?}", temp_staging_dir);
                        let opts = fs_extra::dir::CopyOptions::new()
                            .overwrite(true)
                            .content_only(true);

                        if let Err(e) = fs_extra::dir::copy(&update_source_folder, &temp_staging_dir, &opts) {
                            eprintln!("ERROR: Failed to copy new files: {}", e);
                            eprintln!("Cleaning up temp staging directory: {:?}", temp_staging_dir);
                            let _ = fs::remove_dir_all(&temp_staging_dir);
                            eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                            let _ = fs::remove_dir_all(&temp_path);
                            return;
                        }

                        // Collect list of files in the package (for preservation logic)
                        let new_files = match Self::collect_package_files(&update_source_folder) {
                            Ok(files) => files,
                            Err(e) => {
                                eprintln!("ERROR: Failed to collect package files: {}", e);
                                eprintln!("Cleaning up temp staging directory: {:?}", temp_staging_dir);
                                let _ = fs::remove_dir_all(&temp_staging_dir);
                                eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                                let _ = fs::remove_dir_all(&temp_path);
                                return;
                            }
                        };

                        println!("Package contains {} files", new_files.len());

                        // Merge preserved files from current version
                        println!("Merging preserved files from current version...");
                        match Self::merge_preserved_files(&app_dir_clone, &temp_staging_dir, &new_files) {
                            Ok(count) => {
                                println!("Preserved {} files from current version", count);
                            }
                            Err(e) => {
                                eprintln!("ERROR: Failed to merge preserved files: {}", e);
                                eprintln!("Cleaning up temp staging directory: {:?}", temp_staging_dir);
                                let _ = fs::remove_dir_all(&temp_staging_dir);
                                eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                                let _ = fs::remove_dir_all(&temp_path);
                                return;
                            }
                        }

                        // Get rollback directory from settings (in temp)
                        let rollback_dir = settings.rollback_path.clone();
                        println!("Staging directory: {:?}", temp_staging_dir);
                        println!("Rollback directory: {:?}", rollback_dir);

                        // Perform directory swap (will use file-by-file for cross-filesystem)
                        if let Err(e) = Self::swap_with_fallback(&app_dir_clone, &temp_staging_dir, &rollback_dir) {
                            eprintln!("ERROR: Directory swap failed: {}", e);
                            eprintln!("Cleaning up temp staging directory: {:?}", temp_staging_dir);
                            let _ = fs::remove_dir_all(&temp_staging_dir);
                            eprintln!("Cleaning up temp extraction folder: {}", temp_path.display());
                            let _ = fs::remove_dir_all(&temp_path);
                            return;
                        }

                        // Clean up temp staging directory
                        println!("Cleaning up temp staging directory: {:?}", temp_staging_dir);
                        let _ = fs::remove_dir_all(&temp_staging_dir);

                        // Note: No update tracking for this method (no mark_update_applied call)

                        // Clean up temp extraction folder
                        println!("Cleaning up temp extraction folder: {}", temp_path.display());
                        let _ = fs::remove_dir_all(&temp_path);

                        // Update completed successfully
                        println!("Update applied successfully!");
                        println!("  Active version: {:?}", app_dir_clone);
                        println!("  Rollback available: {:?}", rollback_dir);

                        // Restart the app
                        if settings.app_is_systemd_service {
                            // Restart via systemd
                            if let Some(ref service_name) = settings.app_service_name {
                                println!("Starting systemd service '{}'...", service_name);
                                match Command::new("systemctl")
                                    .arg("start")
                                    .arg(service_name)
                                    .output() {
                                    Ok(output) => {
                                        if output.status.success() {
                                            println!("Successfully started service '{}'", service_name);
                                        } else {
                                            eprintln!("ERROR: Failed to start service '{}': {}",
                                                service_name,
                                                String::from_utf8_lossy(&output.stderr));
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("ERROR: Failed to execute systemctl start: {}", e);
                                    }
                                }
                            } else {
                                eprintln!("ERROR: app_is_systemd_service is true but app_service_name is not set!");
                            }
                        } else {
                            // Direct process spawn
                            println!("Launching '{:?}' in directory '{:?}'...", executable_path_clone, app_dir_clone);
                            match local_command {
                                None => {
                                    let _app = Command::new(&executable_path_clone)
                                        .current_dir(&app_dir_clone)
                                        .stdin(Stdio::null())
                                        .stdout(Stdio::null())
                                        .stderr(Stdio::null())
                                        .process_group(0)
                                        .spawn()
                                        .expect("Failed to start process");
                                },
                                Some(cmd) => {
                                    let _app = Command::new(cmd)
                                        .arg(&executable_path_clone)
                                        .current_dir(&app_dir_clone)
                                        .stdin(Stdio::null())
                                        .stdout(Stdio::null())
                                        .stderr(Stdio::null())
                                        .process_group(0)
                                        .spawn()
                                        .expect("Failed to start process");
                                },
                            }
                        }

                        return;
                    }
                }
            }
        });

        Ok(1)
    }

    fn _extract_update_to_location(_update: Arc<Mutex<UpdateDescriptor>>, file_name: String, destination_root: &String) -> Result<u64, String> {
//            let mut d = update.lock().unwrap();

            let zip_file = File::open(&file_name)
                .map_err(|e| format!("Failed to open zip file '{}': {}", file_name, e))?;
            let mut archive = ZipArchive::new(zip_file)
                .map_err(|e| format!("Failed to read zip archive: {}", e))?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)
                    .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;
                let outpath = Path::new(&destination_root).join(file.name());
                if (&*file.name()).ends_with('/') {
                    std::fs::create_dir_all(&outpath)
                        .map_err(|e| format!("Failed to create directory '{}': {}", outpath.display(), e))?;
                }
                else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(&p)
                                .map_err(|e| format!("Failed to create parent directory '{}': {}", p.display(), e))?;
                        }
                    }
                    let mut outfile = File::create(&outpath)
                        .map_err(|e| format!("Failed to create output file '{}': {}", outpath.display(), e))?;
                    std::io::copy(&mut file, &mut outfile)
                        .map_err(|e| format!("Failed to write to file '{}': {}", outpath.display(), e))?;
                }
            };

            Ok(1)

/*            
                // mark as "applied"
                d.applied = true;

                // update file
                self.save_or_update(&d);
*/
    }

    fn extract_package_to_location(&self, package_path: String, destination_root: &String) -> Result<u64, String> {
        let zip_file = File::open(&package_path)
            .map_err(|e| format!("Failed to open package '{}': {}", package_path, e))?;
        let mut archive = ZipArchive::new(zip_file)
            .map_err(|e| format!("Failed to read package archive: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Failed to read archive entry {}: {}", i, e))?;
            let outpath = Path::new(&destination_root).join(file.name());
            if (&*file.name()).ends_with('/') {
                std::fs::create_dir_all(&outpath)
                    .map_err(|e| format!("Failed to create directory '{}': {}", outpath.display(), e))?;
            }
            else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(&p)
                            .map_err(|e| format!("Failed to create parent directory '{}': {}", p.display(), e))?;
                    }
                }
                let mut outfile = File::create(&outpath)
                    .map_err(|e| format!("Failed to create file '{}': {}", outpath.display(), e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to copy file data to '{}': {}", outpath.display(), e))?;
            }
        }

        Ok(1)
    }

    async fn _extract_app_update(&self, id: &String, destination_root: String) -> Result<u64, String> {
        let update = self.updates.get(id);
        match update {
            Some(u) => {
                let mut d = match u.lock() {
                    Ok(descriptor) => descriptor,
                    Err(e) => {
                        return Err(format!("Failed to lock update descriptor: {}", e));
                    }
                };

                let file_name = format!("{}/{}/update.mpak", self.store_root_folder.display(), d.mpak_id);

                let zip_file = File::open(&file_name)
                    .map_err(|e| format!("Failed to open update package '{}': {}", file_name, e))?;
                let mut archive = ZipArchive::new(zip_file)
                    .map_err(|e| format!("Failed to read archive: {}", e))?;

                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)
                        .map_err(|e| format!("Failed to read archive entry {}: {}", i, e))?;
                    let outpath = Path::new(&destination_root).join(file.name());
                    if (&*file.name()).ends_with('/') {
                        std::fs::create_dir_all(&outpath)
                            .map_err(|e| format!("Failed to create directory '{}': {}", outpath.display(), e))?;
                    }
                    else {
                        if let Some(p) = outpath.parent() {
                            if !p.exists() {
                                std::fs::create_dir_all(&p)
                                    .map_err(|e| format!("Failed to create parent directory '{}': {}", p.display(), e))?;
                            }
                        }
                        let mut outfile = File::create(&outpath)
                            .map_err(|e| format!("Failed to create output file '{}': {}", outpath.display(), e))?;
                        std::io::copy(&mut file, &mut outfile)
                            .map_err(|e| format!("Failed to copy file to '{}': {}", outpath.display(), e))?;
                    }
                }
            
                // mark as "applied"
                d.applied = Some(true);

                // update file
                self.save_or_update(&d);

                // TODO: return something meaningful?
                Ok(1)        
            },
            None => {

                Err(format!("Update {} not known", id))
            }
        }
    }

    pub fn set_jwt(&mut self, jwt: String) {
        self.jwt = jwt;
    }

    pub async fn retrieve_update(&self, id: &String) -> Result<u64, String> {
        
        // is this an update we know about?
        let update = self.updates.get(id);
        match update {
            Some(u) => {
               let mut d = match u.lock() {
                   Ok(descriptor) => descriptor,
                   Err(e) => {
                       return Err(format!("Failed to lock update descriptor: {}", e));
                   }
               };

                let mut sanitized_url = (&d.mpak_download_url).to_string();
                if !sanitized_url.starts_with("http") {
                    // TODO: support auth/https
                    sanitized_url.insert_str(0, "http://");

                }

                let client = reqwest::Client::new();

                let auth_header = match reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.jwt)) {
                    Ok(header) => header,
                    Err(e) => {
                        return Err(format!("Failed to create auth header: {}", e));
                    }
                };

                match client
                    .get(sanitized_url)
                    .header(reqwest::header::AUTHORIZATION, auth_header)
                    .send()
                    .await 
                {            
                    Ok(response) => {
                        
                        // Check for a successful status code
                        if !response.status().is_success() {
                            println!("Failed to download file: HTTP {}", response.status());
                            return Err(format!("Failed to download file: HTTP {}", response.status()));
                        }                        

                        // determine where to store the mpak - we will extract on apply
                        let file_name = format!("{}/{}/update.mpak", self.store_root_folder.display(), d.mpak_id);

                        // download the update
                        //let s = sanitized_url.clone();
                        //println!("downloading {} to {}", s, file_name);
                        println!("downloading {}", file_name);

                        let mut file = match File::create(&file_name) {
                            Ok(f) => f,
                            Err(e) => {
                                return Err(format!("Failed to create file '{}': {}", file_name, e));
                            }
                        };

                        match response.bytes().await {
                            Ok(data) => {
                                let mut content = Cursor::new(data);
                                let size = match copy(&mut content, &mut file) {
                                    Ok(s) => s,
                                    Err(e) => {
                                        return Err(format!("Failed to write downloaded data to file: {}", e));
                                    }
                                };
                
                                // set the update as retrieved
                                d.retrieved = Some(true);
                
                                // update file
                                self.save_or_update(d.deref());
                
                                // return the size?  file name?  something
                                Ok(size)
        
                            },
                            Err(e) => {
                                return Err(e.to_string());
                            }
                        }                                
                    },
                    Err(e) => {
                        return Err(e.to_string());
                    }
                }
            },
            None => {

                Err(format!("Update {} not known", id))
            }
        }
    }

    fn save_or_update(&self, descriptor: &UpdateDescriptor) {
        println!("{:?}", descriptor);

        // make sure subdir exists
        let mut path = self.store_root_folder.join(&descriptor.mpak_id);
        if ! path.exists() {
            if let Err(e) = fs::create_dir(&path) {
                eprintln!("ERROR: Failed to create update directory '{}': {}", path.display(), e);
                return;
            }
        }

        // serialize
        let json = match serde_json::to_string_pretty(&descriptor) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("ERROR: Failed to serialize descriptor: {}", e);
                return;
            }
        };

        // erase any existing file
        path.push(&Self::UPDATE_INFO_FILE_NAME);

        let mut file = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("ERROR: Failed to open file '{}': {}", path.display(), e);
                    return;
                }
            };

        // save
        if let Err(e) = file.write_all(json.as_bytes()) {
            eprintln!("ERROR: Failed to write to file '{}': {}", path.display(), e);
        }

    }

    fn mark_update_applied(update_id: &String, store_root: &PathBuf) {
        let info_path = store_root.join(update_id).join(Self::UPDATE_INFO_FILE_NAME);

        if !info_path.exists() {
            println!("WARNING: Cannot mark update {} as applied - info file not found", update_id);
            return;
        }

        // Read the existing descriptor
        match File::open(&info_path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                match serde_json::from_reader::<_, UpdateDescriptor>(reader) {
                    Ok(mut descriptor) => {
                        // Mark as applied
                        descriptor.applied = Some(true);

                        // Write back to file
                        let json = match serde_json::to_string_pretty(&descriptor) {
                            Ok(j) => j,
                            Err(e) => {
                                println!("ERROR: Failed to serialize descriptor for {}: {:?}", update_id, e);
                                return;
                            }
                        };
                        let mut file = match OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(&info_path) {
                                Ok(f) => f,
                                Err(e) => {
                                    println!("ERROR: Failed to open descriptor file for {}: {:?}", update_id, e);
                                    return;
                                }
                            };
                        if let Err(e) = file.write_all(json.as_bytes()) {
                            println!("ERROR: Failed to write descriptor for {}: {:?}", update_id, e);
                            return;
                        }

                        println!("Marked update {} as applied", update_id);
                    }
                    Err(err) => {
                        println!("ERROR: Failed to parse descriptor for {}: {:?}", update_id, err);
                    }
                }
            }
            Err(err) => {
                println!("ERROR: Failed to open descriptor file for {}: {:?}", update_id, err);
            }
        }
    }
}