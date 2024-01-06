use walkdir::WalkDir;

fn main() {
    let url = "https://github.com/rust-lang/crates.io-index.git";
    let repo_path = "./repo_clone";
    match git2::Repository::clone(url, repo_path) {
        Ok(_) => (),
        Err(e) => {if e.code() != git2::ErrorCode::Exists {
            panic!("failed to clone: {}", e)
        }},
    };
    let max_dependencies: Option<(String, Vec::<String>)> = None;
    let max_features: Option<(String, Vec::<String>)> = None;
    let max_versions: Option<(String, Vec::<String>)> = None;
    let dependants_map: std::collections::HashMap<String, Vec::<String>>;
    let max_delendants: Option<(String, Vec::<String>)> = None;

    for entry in walkdir::WalkDir::new(repo_path)
                           .into_iter()
                           .filter_entry(|e| !e.file_name()
                                                          .to_str()
                                                          .map(|s| s.starts_with('.'))
                                                          .unwrap_or(false))
                           .filter_map(|e| e.ok())
                           .filter(|e| e.file_type().is_file()) {
        let path_opt = entry.path().to_str();
        if path_opt.is_none() { eprintln!("error encountered while getting the path \"{:?}\"", entry); continue; }
        let contents_res = std::fs::read_to_string(path_opt.unwrap());
        if contents_res.is_err() { eprintln!("error encountered while reading file \"{}\", {}", path_opt.unwrap(), contents_res.unwrap_err()); continue; }
        let contents = contents_res.unwrap();
        println!("{:?}", entry);
    }
}