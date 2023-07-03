use std::ffi::OsStr;
use std::sync::{Mutex, Arc};
use std::thread::{self, sleep};
use std::time::Duration;
use std::{collections::HashMap, ops::Deref};
use std::path::{Path, PathBuf};
use std::fs::{self, OpenOptions, File};
use std::io::{Write, Cursor, copy, BufReader};
use zip::{ZipArchive};

use crate::{cloud_settings::CloudSettings, update_descriptor::UpdateDescriptor};

pub struct UpdateStore {
    _settings: CloudSettings,
    store_directory: PathBuf,
    updates: HashMap<String, Arc<Mutex<UpdateDescriptor>>>
}

impl UpdateStore {
    const STORE_ROOT_FOLDER:&str = "/home/ctacke/meadow/updates";
    const UPDATE_INFO_FILE_NAME: &str = "info.json";

    pub fn new(settings: CloudSettings) -> UpdateStore {
        let mut store = UpdateStore {
            _settings : settings,
            store_directory: PathBuf::from(Self::STORE_ROOT_FOLDER),
            updates: HashMap::new()
        };
        
        println!("Update data will be stored in '{:?}'", store.store_directory);

        if ! store.store_directory.exists() {
            fs::create_dir(&store.store_directory).unwrap();
        }
        else {
            // TODO: load all existing update descriptors
            for entry in fs::read_dir(&store.store_directory).unwrap() {
                match entry {
                    Ok(e) => {
                        if e.path().is_dir() {
                            // it's a likely update folder, but look for (and parse) an info file to be sure
                            for entry in fs::read_dir(e.path()).unwrap() {
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
                                    Err(_e) => {
                                        // TODO: ???
                                    }
                                }
                            }
                        }
                    },
                    Err(_e) => {
                        // TODO: ???
                    }
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

    pub fn clear(&mut self) {
        self.updates.clear();
    }

    pub async fn apply_update(&self, id: &String, app_path: &PathBuf, pid: i32) -> Result<u64, String> {
        let p = app_path.clone();
        let update = self.updates.get(id).unwrap().clone();

        // extract the update to a temp location
        let d = update.lock().unwrap();
        let package_path = format!("{}/{}/update.mpak", Self::STORE_ROOT_FOLDER, d.mpak_id);
        let update_temp_path = format!("{}/{}/tmp", Self::STORE_ROOT_FOLDER, d.mpak_id);
        self.extract_package_to_location(package_path, &update_temp_path).unwrap();

        // make sure it's a valid app update (i.e. has an `app` folder)
        let update_source_folder = Path::new(&update_temp_path).join("app");
        if !update_source_folder.is_dir() {
            return Err("Package does not contain a valid Application update".to_string());
        }

        // spawn a thread to wait for app shutdown

        thread::spawn(move || {
            let application_folder = p.parent().unwrap().to_str().unwrap();
            let app = p.file_name().unwrap().to_str().unwrap();
            let proc_folder = format!("/proc/{}", pid);
            let proc_path = Path::new(&proc_folder);

            println!("Caller is '{}' (PID {}) running from '{}'", app, pid, application_folder);

            loop {
                // there's probably a better way to do this, but I can't find it
                // wait::waitpid only works for child processes

                match proc_path.is_dir() {
                    true => {
                        println!("'{}' is still alive", app);

                        // todo: put in a timeout escape hatch here

                        sleep(Duration::from_millis(1000));
                    },
                    _ => {
                        println!("'{}' exited", &app);

                        // todo: copy existing app binaries to a rollback folder

                        // move the update to the app folder
                        let opts = fs_extra::dir::CopyOptions::new()
                        .overwrite(true)
                        .content_only(true);

                        fs_extra::dir::copy(
                            &update_source_folder,
                            application_folder,
                            &opts
                            ).unwrap();

                        // todo: update the descriptor to "applied"

                        // todo: restart the app
                    }
                }
            }
        });

        Ok(1)
    }

    fn _extract_update_to_location(update: Arc<Mutex<UpdateDescriptor>>, file_name: String, destination_root: &String) -> Result<u64, String> {
            let mut d = update.lock().unwrap();

            let zip_file = File::open(file_name).unwrap();
            let mut archive = ZipArchive::new(zip_file).unwrap();
        
            for i in 0..archive.len() {
                let mut file = archive.by_index(i).unwrap();
                let outpath = Path::new(&destination_root).join(file.name());
                if (&*file.name()).ends_with('/') {
                    std::fs::create_dir_all(&outpath).unwrap();
                } 
                else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(&p).unwrap();
                        }
                    }
                    let mut outfile = File::create(&outpath).unwrap();
                    std::io::copy(&mut file, &mut outfile).unwrap();
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
        let zip_file = File::open(package_path).unwrap();
        let mut archive = ZipArchive::new(zip_file).unwrap();
    
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = Path::new(&destination_root).join(file.name());
            if (&*file.name()).ends_with('/') {
                std::fs::create_dir_all(&outpath).unwrap();
            } 
            else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(&p).unwrap();
                    }
                }
                let mut outfile = File::create(&outpath).unwrap();
                std::io::copy(&mut file, &mut outfile).unwrap();
            }
        }

        Ok(1)
    }

    async fn _extract_app_update(&self, id: &String, destination_root: String) -> Result<u64, String> {
        let update = self.updates.get(id);
        match update {
            Some(u) => {
                let mut d = u.lock().unwrap();

                let file_name = format!("{}/{}/update.mpak", Self::STORE_ROOT_FOLDER, d.mpak_id);

                let zip_file = File::open(file_name).unwrap();
                let mut archive = ZipArchive::new(zip_file).unwrap();
            
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i).unwrap();
                    let outpath = Path::new(&destination_root).join(file.name());
                    if (&*file.name()).ends_with('/') {
                        std::fs::create_dir_all(&outpath).unwrap();
                    } 
                    else {
                        if let Some(p) = outpath.parent() {
                            if !p.exists() {
                                std::fs::create_dir_all(&p).unwrap();
                            }
                        }
                        let mut outfile = File::create(&outpath).unwrap();
                        std::io::copy(&mut file, &mut outfile).unwrap();
                    }
                }
            
                // mark as "applied"
                d.applied = true;

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

    pub async fn retrieve_update(&self, id: &String) -> Result<u64, String> {
        
        // is this an update we know about?
        let update = self.updates.get(id);
        match update {
            Some(u) => {
               let mut d = u.lock().unwrap();

                let mut sanitized_url = (&d.mpak_download_url).to_string();
                if !sanitized_url.starts_with("http") {
                    // TODO: support auth/https
                    sanitized_url.insert_str(0, "http://");

                }

                match reqwest::get(&sanitized_url).await {
                    Ok(response) => {
                        // determine where to store the mpak - we will extract on apply
                        let file_name = format!("{}/{}/update.mpak", Self::STORE_ROOT_FOLDER, d.mpak_id);

                        // download the update
                        println!("downloading {} to {}", sanitized_url, file_name);
                        
                        let mut file = File::create(file_name).unwrap();

                        match response.bytes().await {
                            Ok(data) => {
                                let mut content = Cursor::new(data);
                                let size = copy(&mut content, &mut file).unwrap();
                
                                // set the update as retrieved
                                d.retrieved = true;
                
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
        let mut path = Path::new(Self::STORE_ROOT_FOLDER).join(&descriptor.mpak_id);
        if ! path.exists() {
            fs::create_dir(&path).unwrap();
        }

        // serialize
        let json = serde_json::to_string_pretty(&descriptor).unwrap();

        // erase any existing file
        path.push(&Self::UPDATE_INFO_FILE_NAME);

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .unwrap();

        // save
        file.write_all(json.as_bytes()).unwrap();

    }
}