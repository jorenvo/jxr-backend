#[macro_use]
extern crate rocket;
use std::{env, fs, process::Command, sync::Mutex};

use rocket::{routes, State};
use serde_json::json;

#[derive(Debug)]
struct JXRState {
    code_dir: String,
    globs: Vec<String>,
    global_rg_lock: Mutex<bool>,
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

    if options.path.is_some() && json["type"].as_str() == Some("match") {
        if json["data"]["path"]["text"]
            .as_str()
            .expect("result didn't have path")
            .contains(options.path.as_ref().unwrap())
        {
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

fn run_ripgrep(config: &JXRState, tree: &str, options: &Options) -> String {
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

    let mut results: Vec<serde_json::Value> = vec![];
    let output = command.output().expect("failed to execute process").stdout;
    let output_utf8 = String::from_utf8(output).expect("rg did not return valid utf8");
    for line in output_utf8.lines() {
        if let Some(result) = parse_result(line, options) {
            let result_type = result["type"].as_str();

            // summary will be last
            if result_type == Some("begin") || result_type == Some("summary") {
                pop_if_empty_begin(&mut results);
            }

            results.push(result);
        }
    }

    json!(results).to_string()
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

#[get("/search?<tree>&<query>")]
fn search(config: &State<JXRState>, tree: &str, query: &str) -> String {
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
fn trees(config: &State<JXRState>) -> String {
    let paths = fs::read_dir(&config.code_dir).unwrap();
    let paths: Vec<String> = paths
        .map(|p| p.unwrap().file_name().to_str().unwrap().to_string())
        .collect();

    json!(paths).to_string()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(JXRState {
            code_dir: env::var("JXR_CODE_DIR").expect("JXR_CODE_DIR is not set"),
            globs: vec!["!*.po".to_string(), "!*.pot".to_string()],
            global_rg_lock: Mutex::new(false),
        })
        .mount("/", routes![search, trees])
}
