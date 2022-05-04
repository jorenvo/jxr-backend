#[macro_use]
extern crate rocket;
use std::io::Error;
use std::{env, fs, process::Command, sync::Mutex};

use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::{routes, State};
use serde_json::json;

#[derive(Debug)]
struct JXRState {
    max_matches: usize,
    code_dir: String,
    globs: Vec<String>,
    global_rg_lock: Mutex<()>,
}

// TODO
// - error handling
// - check if we could construct our own deserialization types for serde
// - remove global_rg_lock

fn parse_result(line: &str, options: &Options) -> Option<serde_json::Value> {
    let json: serde_json::Value = serde_json::from_str(line).expect("json was not well-formatted");

    // always skip ends
    if json["type"].as_str() == Some("end") {
        return None;
    }

    if json["type"].as_str() == Some("match") {
        let has_text = json["data"]["lines"].get("text").is_some();
        let in_path = options.path.is_none()
            || json["data"]["path"]["text"]
                .as_str()
                .expect("result didn't have path")
                .contains(options.path.as_ref().unwrap());
        if has_text && in_path {
            return Some(json);
        } else {
            return None;
        }
    }

    Some(json)
}

fn pop_if_empty_begin(results: &mut Vec<serde_json::Value>) {
    if let Some(last) = results.last() {
        if last["type"].as_str() == Some("begin") {
            results.pop();
        }
    }
}

fn get_ripgrep_output(command: &mut Command) -> Result<Vec<u8>, Custom<String>> {
    let output = command.output().expect("failed to execute process");
    if output.status.code().is_none() || output.status.code() == Some(2) {  // rg returns 1 when no matches are found
        return Err(Custom(
            Status::InternalServerError,
            format!(
                "Ripgrep failed: {}",
                String::from_utf8(output.stderr).unwrap()
            ),
        ));
    }

    Ok(output.stdout)
}

fn convert_to_utf8(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("rg did not return valid utf8")
}

fn run_ripgrep(config: &JXRState, tree: &str, options: &Options) -> Result<String, Custom<String>> {
    let mut command = Command::new("rg");

    // TODO: directory traversal attack!
    command.current_dir(format!("{}/{}", config.code_dir, tree));

    command.arg("--json");

    for glob in &config.globs {
        command.arg("--glob");
        command.arg(glob);
    }

    if let Some(filetype) = options.filetype.as_ref() {
        command.arg("--type");
        command.arg(filetype);
    }

    // TODO: error properly here
    command.arg(options.pattern.as_ref().expect("no search pattern"));

    println!("Running ripgrep: {:?}", command);
    let output = get_ripgrep_output(&mut command).unwrap();
    let output_utf8 = convert_to_utf8(output);

    let mut results: Vec<serde_json::Value> = vec![];
    let mut truncated = false;
    let mut matches = 0;
    for line in output_utf8.lines() {
        if let Some(result) = parse_result(line, options) {
            let result_type = result["type"].as_str();

            if matches >= config.max_matches {
                truncated = true;
                break;
            }

            match result_type {
                Some("summary") => {
                    break;
                }
                Some("match") => {
                    matches += 1;
                }
                Some("begin") => {
                    pop_if_empty_begin(&mut results);
                }
                _ => {}
            }

            results.push(result);
        }
    }

    let mut summary = parse_result(
        output_utf8.lines().last().expect("no summary line"),
        options,
    )
    .expect("summary line didn't parse");
    summary["data"]["stats"]["truncated"] = json!(truncated);
    results.push(summary);

    if truncated {
        println!("Truncated results to {}", config.max_matches);
    }

    Ok(json!(results).to_string())
}

#[derive(Default, Debug)]
struct Options {
    path: Option<String>,
    filetype: Option<String>,
    pattern: Option<String>,
}

fn parse_options(query: &str) -> Options {
    const ID_PATH: &str = "path:";
    const ID_TYPE: &str = "type:";
    const ID_EXT: &str = "ext:";
    let mut options: Options = Default::default();

    for part in query.split_whitespace() {
        if let Some(path) = part.strip_prefix(ID_PATH) {
            options.path = Some(path.to_string());
        } else if let Some(type_) = part.strip_prefix(ID_TYPE) {
            options.filetype = Some(type_.to_string());
        } else if let Some(type_) = part.strip_prefix(ID_EXT) {
            // TODO: implement with glob later
            options.filetype = Some(type_.to_string());
        } else {
            let mut pattern: String = options.pattern.as_ref().unwrap_or(&String::new()).clone();

            if !pattern.is_empty() {
                pattern += " ";
            }
            pattern += part;

            options.pattern = Some(pattern);
        }
    }

    println!("Searching with options: {:?}", options);

    options
}

fn http_error(msg: &str) -> Result<String, Custom<String>> {
    Err(Custom(
        Status::InternalServerError,
        format!("Git failed: {}", msg),
    ))
}

#[get("/search?<tree>&<query>")]
fn search(config: &State<JXRState>, tree: &str, query: &str) -> Result<String, Custom<String>> {
    let options = parse_options(query);
    let _lock = config
        .global_rg_lock
        .lock()
        .expect("unlocking global rg lock failed");
    let grep_json = run_ripgrep(config, tree, &options);

    println!("Finished searching for {}", query);
    grep_json
}

#[get("/trees")]
fn trees(config: &State<JXRState>) -> Result<String, Error> {
    let mut paths: Vec<String> = fs::read_dir(&config.code_dir)?
        .filter(|p| p.is_ok())
        .map(|p| p.unwrap())
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .map(|entry| entry.file_name().to_str().unwrap().to_string())
        .filter(|dir_name| !dir_name.starts_with('.'))
        .collect();

    paths.sort();

    Ok(json!(paths).to_string())
}

fn find_repo(path: &str) -> Result<String, ()> {
    let mut parent = String::new();

    for part in path.split('/') {
        parent.push_str(part);
        parent.push('/');
        if fs::read_dir(parent.clone() + "/.git").is_ok() {
            return Ok(parent);
        }
    }

    Err(())
}

#[get("/gitroot?<path>")]
fn git_root(config: &State<JXRState>, path: &str) -> Result<String, Custom<String>> {
    // TODO: directory traversal attack!
    let full_path = format!("{}/{}", config.code_dir, path);
    let repo_path = find_repo(&full_path);
    if repo_path.is_err() {
        return http_error("no git repo in tree");
    }

    let full_path = repo_path.unwrap();
    println!(
        "Stripping {} from {}",
        &format!("{}/", config.code_dir),
        full_path
    );
    let stripped = full_path
        .strip_prefix(&format!("{}/", config.code_dir))
        .unwrap();

    Ok(json!(stripped).to_string())
}

#[get("/head?<path>")]
fn git_head(config: &State<JXRState>, path: &str) -> Result<String, Custom<String>> {
    let mut command = Command::new("git");

    // TODO: directory traversal attack!
    let full_path = format!("{}/{}", config.code_dir, path);
    let repo_path = find_repo(&full_path);
    if repo_path.is_err() {
        return http_error("no git repo in tree");
    }

    command.current_dir(repo_path.unwrap());

    command.args(["rev-parse", "HEAD"]);

    let output = command.output().expect("failed to execute process");
    if !output.status.success() {
        return http_error(&String::from_utf8(output.stderr).unwrap());
    }

    Ok(json!(String::from_utf8(output.stdout)
        .expect("rg did not return valid utf8")
        .trim())
    .to_string())
}

#[get("/github?<path>")]
fn github(config: &State<JXRState>, path: &str) -> Result<String, Custom<String>> {
    let mut command = Command::new("git");

    // TODO: directory traversal attack!
    let full_path = format!("{}/{}", config.code_dir, path);
    let repo_path = find_repo(&full_path);
    if repo_path.is_err() {
        return http_error("no git repo in tree");
    }

    command.current_dir(repo_path.unwrap());

    command.args(["config", "--get", "remote.origin.url"]);

    let output = command.output().expect("failed to execute process");
    if !output.status.success() {
        return http_error(&String::from_utf8(output.stderr).unwrap());
    }

    // TODO: handle public remote URL
    // Will return something like:
    // git@github.com:jorenvo/jxr-backend.git
    let remote = String::from_utf8(output.stdout).expect("rg did not return valid utf8");
    Ok(json!(remote
        .replace("git@github.com:", "")
        .replace(".git", "")
        .trim())
    .to_string())
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(JXRState {
            max_matches: 1_000,
            code_dir: env::var("JXR_CODE_DIR").expect("JXR_CODE_DIR is not set"),
            globs: vec!["!*.po".to_string(), "!*.pot".to_string()],
            global_rg_lock: Mutex::new(()),
        })
        .mount("/", routes![search, trees, git_head, git_root, github])
}
