use std::os::windows::fs::OpenOptionsExt;
use std::sync::{Arc, Mutex, MutexGuard};
use std::collections::HashMap;
use threadpool::ThreadPool;
use std::io::Write;

fn main() {
    let start_time = std::time::Instant::now();

    let url: &str = "https://github.com/rust-lang/crates.io-index.git";
    let repo_path: &str = "./repo_clone";
    match git2::Repository::clone(url, repo_path) {
        Ok(_) => (),
        Err(e) => {if e.code() != git2::ErrorCode::Exists {
            panic!("failed to clone: {}", e)
        }},
    };

    let pool: ThreadPool = /*ThreadPool::default()*/ThreadPool::new(4);
    
    type Amosvs = Arc<Mutex<Option<(String, Vec<String>)>>>;
    type Amosu = Arc<Mutex<Option<(String, usize)>>>;
    type Amos = Arc<Mutex<Option<String>>>;
    type Amhsvs = Arc<Mutex<HashMap<String, Vec<String>>>>;

    let max_dependencies_am: Amosvs = Arc::new(Mutex::new(None));
    let dependants_map_am: Amhsvs = Arc::new(Mutex::new(HashMap::<String, Vec::<String>>::new()));
    let max_dependants_am: Amos = Arc::new(Mutex::new(None));
    let max_features_am: Amosvs = Arc::new(Mutex::new(None));
    let max_versions_am: Amosu = Arc::new(Mutex::new(None));

    // frequently used constants
    let name_json_str: &str = "\"name\":\"";
    let features_json_str: &str = "\"features\":{";

    for entry in walkdir::WalkDir::new(repo_path)
                           .into_iter()
                           .filter_entry(|e: &walkdir::DirEntry| !e.file_name()
                                                                   .to_str()
                                                                   .map(|s: &str| s.starts_with('.') || s == "config.json")
                                                                   .unwrap_or(false))
                           .filter_map(|e: Result<walkdir::DirEntry, walkdir::Error>| e.ok())
                           .filter(|e: &walkdir::DirEntry| e.file_type().is_file()) {

        let max_dependencies_amc: Amosvs = Arc::clone(&max_dependencies_am);
        let dependants_map_amc: Amhsvs = Arc::clone(&dependants_map_am);
        let max_dependants_amc: Amos = Arc::clone(&max_dependants_am);
        let max_features_amc: Amosvs = Arc::clone(&max_features_am);
        let max_versions_amc: Amosu = Arc::clone(&max_versions_am);

        pool.execute(move || {

            let path: &str = {
                let opt: Option<&str> = entry.path().to_str();
                if opt.is_none() { eprintln!("failed to get the path \"{:?}\"", entry); return; }
                opt.unwrap()
            };

            let contents: String = {
                let res: Result<String, std::io::Error> = std::fs::read_to_string(path);
                if res.is_err() { eprintln!("failed read file \"{}\", {}", path, res.unwrap_err()); return; }
                res.unwrap()
            };

            let last_line: &str = contents.lines().last().unwrap();

            let (name, dep_start): (&str, usize) = {
                let begin_offset: usize = {
                    let opt: Option<usize> = last_line.find(name_json_str);
                    if opt.is_none() { eprintln!("failed to read the beginning of the \"name\" section form the .json file: {}", path); 
                                        eprintln!("{}", last_line); return; }
                    opt.unwrap() + name_json_str.len()
                };

                let end_offset: usize = {
                    let opt: Option<usize> = last_line[begin_offset..].find('\"');
                    if opt.is_none() { eprintln!("failed to read the name from the .json file: {}", path); eprintln!("{}", &last_line[begin_offset..]); return; }
                    opt.unwrap()
                };

                (&last_line[begin_offset..begin_offset + end_offset], begin_offset + end_offset + "\"".len())
            };

            let dependencies: Vec<&str> = last_line[dep_start..]
                .match_indices(name_json_str)
                .filter_map(|(name_offset, _): (usize, _)| {
                    let name_start: usize = dep_start + name_offset + name_json_str.len();
                    let name_length_opt: Option<usize> = last_line[name_start..].find('\"');
                    if name_length_opt.is_none() { eprintln!("invalid .json file ({}) does not have a name quote closed:", path);
                                                   eprintln!("{}", &last_line[name_start..]); return None; }
                    Some(&last_line[name_start..name_start + name_length_opt.unwrap()])
              })
                .inspect(|&s| {
                    let mut dependants_map: MutexGuard<HashMap<String, Vec<String>>> = dependants_map_amc.lock().unwrap();
                    let dependants_count =  {
                        let mut opt: Option<&mut Vec<String>> = dependants_map.get_mut(s);
                        if opt.is_none() {
                            dependants_map.insert((*s).to_string(), Vec::from([(name).to_string()]));
                            1
                        } else {
                            opt.as_mut().unwrap().push((name).to_string());
                            opt.unwrap().len()
                        }
                    };
                    let mut max_dependants: MutexGuard<Option<String>> = max_dependants_amc.lock().unwrap();
                    if max_dependants.is_none() || dependants_count > dependants_map.get(max_dependants.as_ref().unwrap()).unwrap().len() {
                        *max_dependants = Some(s.to_string());
                    }
              })
                .collect();

            let features: Vec<&str> = {
                let features_start: usize = {
                    let opt: Option<usize> = last_line.find(features_json_str);
                    if opt.is_none() { eprintln!("failed to locate the feaures list in: {}", path); eprintln!("{}", last_line); return; }
                    opt.unwrap()
                };

                let features_end: usize = {
                    let opt: Option<usize> = last_line[features_start..].find('}');
                    if opt.is_none() { eprintln!("invalid .json file ({}) does not have the features list closed", path); eprintln!("{}", last_line); return; }
                    opt.unwrap()
                };

                let mut split_rev = last_line[features_start..features_start + features_end].rsplit("\":[");
                split_rev.next();
                split_rev.filter_map(|e: &str| {
                            let feature_begin_opt: Option<usize> = e.rfind('\"');
                            if feature_begin_opt.is_none() { eprintln!("invalid .json file ({}) does not have closed quotes for a feature name", path);
                                                             eprintln!("{}", e); return None; }
                            Some(&e[feature_begin_opt.unwrap() + 1..])
                       })
                         .collect()
            };

            let version_count: usize = contents.lines().count();

            {
                let mut max_dependencies: MutexGuard<Option<(String, Vec<String>)>> = max_dependencies_amc.lock().unwrap();
                if max_dependencies.is_none() || max_dependencies.as_ref().unwrap().1.len() < dependencies.len() {
                    *max_dependencies = Some((name.to_string(), dependencies.into_iter().map(|s: &str| s.to_string() ).collect()));
                }
            }

            {
                let mut max_features: MutexGuard<Option<(String, Vec<String>)>> = max_features_amc.lock().unwrap();
                if max_features.is_none() || max_features.as_ref().unwrap().1.len() < features.len() {
                    *max_features = Some((name.to_string(), features.into_iter().map(|s: &str| s.to_string() ).collect()));
                }
            }

            {
                let mut max_versions: MutexGuard<Option<(String, usize)>> = max_versions_amc.lock().unwrap();
                if max_versions.is_none() || max_versions.as_ref().unwrap().1 < version_count {
                    *max_versions = Some((name.to_string(), version_count));
                }
            }

            // eprintln!("processed: {}", path);

        });
    }

    pool.join();

    let mut output_file: std::fs::File = {
        let res: Result<std::fs::File, std::io::Error> = std::fs::OpenOptions::new()
                                                                              .create(true)
                                                                              .access_mode(0o006)
                                                                              .append(true)
                                                                              .open("results.txt");
        if res.is_err() { eprintln!("failed to open the output file, {}", res.unwrap_err()); return; }
        res.unwrap()
    };
    
    {
        let max_dependencies: MutexGuard<Option<(String, Vec<String>)>> = max_dependencies_am.lock().unwrap();
        let res: Result<(), std::io::Error> = writeln!(output_file, "max dependencies: {:?}", max_dependencies.as_ref().map(|(name, dependencies)| (name, dependencies.len())).unwrap());
        if res.is_err() { eprintln!("failed to write part of the output to the file, {}", res.unwrap_err()); return; }
    }
    {
        let dependants_map: MutexGuard<HashMap<String, Vec<String>>> = dependants_map_am.lock().unwrap();
        let max_dependants: MutexGuard<Option<String>> = max_dependants_am.lock().unwrap();
        let res: Result<(), std::io::Error> = writeln!(output_file, "max dependants: {:?}", dependants_map.get_key_value(max_dependants.as_ref().unwrap()).map(|(name, features)| (name, features.len())).unwrap());
        if res.is_err() { eprintln!("failed to write part of the output to the file, {}", res.unwrap_err()); return; }
    }
    {
        let max_features: MutexGuard<Option<(String, Vec<String>)>> = max_features_am.lock().unwrap();
        let res: Result<(), std::io::Error> = writeln!(output_file, "max features: {:?}", max_features.as_ref().map(|(name, features)| (name, features.len())).unwrap());
        if res.is_err() { eprintln!("failed to write part of the output to the file, {}", res.unwrap_err()); return; }
    }
    {
        let max_versions: MutexGuard<Option<(String, usize)>> = max_versions_am.lock().unwrap();
        let res: Result<(), std::io::Error> = writeln!(output_file, "max versions: {:?}", max_versions.as_ref().unwrap());
        if res.is_err() { eprintln!("failed to write part of the output to the file, {}", res.unwrap_err()); return; }
    }

    println!("time elapsed: {:?}", start_time.elapsed())
}