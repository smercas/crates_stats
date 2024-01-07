use walkdir::WalkDir;

fn main() {
    let url: &str = "https://github.com/rust-lang/crates.io-index.git";
    let repo_path: &str = "./repo_clone";
    match git2::Repository::clone(url, repo_path) {
        Ok(_) => (),
        Err(e) => {if e.code() != git2::ErrorCode::Exists {
            panic!("failed to clone: {}", e)
        }},
    };
    let mut max_dependencies: Option<(String, Vec::<String>)> = None;
    let mut max_features: Option<(String, Vec::<String>)> = None;
    let mut max_versions: Option<(String, Vec::<String>)> = None;
    let mut dependants_map: std::collections::HashMap<String, Vec::<String>>;
    let mut max_dependants: Option<(String, Vec::<String>)> = None;

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
            let path: &str = {
                let opt: Option<&str> = entry.path().to_str();
                if opt.is_none() { eprintln!("failed to get the path \"{:?}\"", entry); continue; }
                opt.unwrap()
            };

            let contents: String = {
                let res: Result<String, std::io::Error> = std::fs::read_to_string(path);
                if res.is_err() { eprintln!("failed read file \"{}\", {}", path, res.unwrap_err()); continue; }
                res.unwrap()
            };

            let last_line: &str = contents.lines().last().unwrap();

            let (name, dep_start): (&str, usize) = {
                let begin_offset: usize = {
                    let opt: Option<usize> = last_line.find(name_json_str);
                    if opt.is_none() { eprintln!("failed to read the beginning of the \"name\" section form the .json file: {}", path); 
                                       eprintln!("{}", last_line); continue; }
                    opt.unwrap() + name_json_str.len()
                };

                let end_offset: usize = {
                    let opt: Option<usize> = last_line[begin_offset..].find('\"');
                    if opt.is_none() { eprintln!("failed to read the name from the .json file: {}", path); eprintln!("{}", &last_line[begin_offset..]); continue; }
                    opt.unwrap()
                };

                (&last_line[begin_offset..begin_offset + end_offset], begin_offset + end_offset + "\"".len())
            };

            let dependencies: Vec<&str> = last_line[dep_start..]
                .match_indices(name_json_str)
                .map(|(name_offset, _): (usize, _)| {
                    let name_start: usize = dep_start + name_offset + name_json_str.len();
                    let name_length_opt: Option<usize> = last_line[name_start..].find('\"');
                    if name_length_opt.is_none() { eprintln!("invalid .json file ({}) does not have a name quote closed:", path);
                                                   eprintln!("{}", &last_line[name_start..]); return None; }
                    return Some(&last_line[name_start..name_start + name_length_opt.unwrap()]);
              }).filter(|s| s.is_some() )
                .map(|s| s.unwrap() )
                .collect();

            let features: Vec<&str> = {
                let features_start: usize = {
                    let opt: Option<usize> = last_line.find(features_json_str);
                    if opt.is_none() { eprintln!("failed to locate the feaures list in: {}", path); eprintln!("{}", last_line); continue; }
                    opt.unwrap()
                };

                let features_end: usize = {
                    let opt: Option<usize> = last_line[features_start..].find('}');
                    if opt.is_none() { eprintln!("invalid .json file ({}) does not have the features list closed", path); eprintln!("{}", last_line); continue; }
                    opt.unwrap()
                };

                let mut split_rev = last_line[features_start..features_start + features_end].rsplit("\":[");
                split_rev.next();
                split_rev.map(|e: &str| {
                        let feature_begin_opt: Option<usize> = e.rfind("\"");
                        if feature_begin_opt.is_none() { eprintln!("invalid .json file ({}) does not have closed quotes for a feature name", path);
                                                         eprintln!("{}", e); return None; }
                        Some(&e[feature_begin_opt.unwrap() + 1..])
                    }).filter(|e| e.is_some())
                    .map(|e| e.unwrap())
                    .collect()
            };

            let version_count: usize = contents.lines().count();

            // if (max_dependencies.is_none() || max_dependencies.as_ref().unwrap().1.len() < dependencies.len()) {
            //     max_dependencies = Some((name.to_string(), dependencies.into_iter().map(|s| s.to_string()).collect::<Vec<String>>()));
            // }

            // if (max_features)

            // break;
    }
}