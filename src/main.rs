#[macro_use]
extern crate rocket;
use std::{fs, process::Command};

use serde_json::json;

// TODO: this should be configurable
const JXR_CODE_DIR: &str = "/Users/jvo/Code/jxr-frontend/dist/jxr-code";

// TODO
// - error handling
// - check if we could construct our own deserialization types for serde

fn parse_result(line: &str, options: &Options) -> Option<serde_json::Value> {
    let json: serde_json::Value = serde_json::from_str(&line).expect("json was not well-formatted");

    if options.path.is_some() && json["type"].as_str().expect("type wasn't a string") == "match" {
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

fn run_ripgrep(tree: &str, options: &Options) -> String {
    let mut command = Command::new("rg");

    // TODO: directory traversal attack!
    command.current_dir(format!("{}/{}", JXR_CODE_DIR, tree));

    command.arg("--json");

    // TODO: error properly here
    command.arg(options.pattern.as_ref().expect("no search pattern"));

    let mut results: Vec<serde_json::Value> = vec![];
    let output = command.output().expect("failed to execute process").stdout;
    let output_utf8 = String::from_utf8(output).expect("rg did not return valid utf8");
    for line in output_utf8.lines() {
        if let Some(result) = parse_result(line, options) {
            results.push(result);
        }
    }

    json!(results).to_string()
}

#[derive(Default)]
struct Options {
    path: Option<String>,
    pattern: Option<String>,
}

fn parse_options(query: &str) -> Options {
    const ID_PATH: &str = "path:";
    let mut options: Options = Default::default();

    for part in query.split_whitespace() {
        if part.starts_with(ID_PATH) {
            options.path = Some(part[ID_PATH.len()..].to_string());
        } else {
            options.pattern = Some(part.to_string());
        }
    }

    options
}

#[get("/search?<tree>&<query>")]
fn search(tree: &str, query: &str) -> String {
    let options = parse_options(query);
    let grep_json = run_ripgrep(tree, &options);

    println!("finished searching for {}", query);
    grep_json
}

#[get("/trees")]
fn trees() -> String {
    let paths = fs::read_dir(JXR_CODE_DIR).unwrap();
    let paths: Vec<String> = paths
        .map(|p| p.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    let mut json_array = "[".to_string();
    for (i, path) in paths.iter().enumerate() {
        json_array.push('"');
        json_array.push_str(path);
        json_array.push('"');

        if i < paths.len() - 1 {
            json_array.push(',');
        }
    }

    json_array.push(']');

    json_array
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![search, trees])
}
