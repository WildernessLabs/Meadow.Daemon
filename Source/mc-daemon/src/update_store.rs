use std::ffi::OsStr;
use std::sync::{Mutex, Arc};
use std::thread::{self, sleep};
use std::time::Duration;
use std::{collections::HashMap, ops::Deref};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::{self, OpenOptions, File};
use std::io::{Write, Cursor, copy, BufReader};
use serde::{Serialize, Deserialize};
use zip::ZipArchive;

use crate::{cloud_settings::CloudSettings, update_descriptor::UpdateDescriptor};

#[derive(Serialize, Deserialize)]
struct MeadowCloudLoginResponseMessage {
    #[serde(alias = "encryptedKey")]
    encrypted_key: String,
    #[serde(alias = "encryptedToken")]
    encrypted_token: String,
    iv: String
}

#[derive(Serialize, Deserialize)]
struct MeadowCloudLoginRequestMessage {
    id: String,
}

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
        let store_root = settings.meadow_root.join("updates");

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
        let update_temp_path = format!("{}/{}/tmp", self.store_root_folder.display(), d.mpak_id);
        if let Err(e) = self.extract_package_to_location(package_path, &update_temp_path) {
            let msg = format!("Failed to extract package: {}", e);
            eprintln!("ERROR: {}", msg);
            return Err(msg);
        }

        // make sure it's a valid app update (i.e. has an `app` folder)
        let update_source_folder = Path::new(&update_temp_path).join("app");
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
                    println!("Cleaning up temp extraction folder: {}", temp_path);
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

                        // todo: copy existing app binaries to a rollback folder

                        // move the update to the app folder
                        let opts = fs_extra::dir::CopyOptions::new()
                        .overwrite(true)
                        .content_only(true);

                        println!("Copying update from '{:?}' to '{}'", update_source_folder, application_folder);

                        match fs_extra::dir::copy(
                            &update_source_folder,
                            application_folder,
                            &opts) {
                                Ok(_) => {
                                    // Mark update as "applied" in descriptor
                                    Self::mark_update_applied(&update_id, &store_root);

                                    // Clean up temp extraction folder
                                    println!("Cleaning up temp extraction folder: {}", temp_path);
                                    let _ = fs::remove_dir_all(&temp_path);

                                    // restart the app
                                    println!("Launching '{:?}'...", p);

                                    match local_command {
                                        None => {
                                            let _app = Command::new(&p)
                                            .spawn()
                                            .expect("Failed to start process");                                
                                        },
                                        Some(cmd) => {
                                            let _app = Command::new(cmd)
                                            .arg(&p)
                                            .spawn()
                                            .expect("Failed to start process");                                
                                        },
                                    }
                                }
                                Err(e) => {
                                    println!("Failed to copy update: {}", e);
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