use serde_json::{Result, Value};
use std::process::Command;

fn run_ripgrep(paths: &[&str], pattern: &str) -> Vec<u8> {
    let mut command = Command::new("rg");

    command.arg("--json");
    command.arg(pattern);

    for path in paths {
        command.arg(path);
    }

    let output = command.output().expect("failed to execute process");
    output.stdout
}

fn main() {
    println!("Hello, world!");

    let grep_json = run_ripgrep(
        &["/Users/jvo/Code/jxr-backend", "/Users/jvo/Code/gb-emu"],
        "const",
    );

    println!("{}", String::from_utf8(grep_json.clone()).unwrap());

    // TODO: find a better way to split
    for line in grep_json.split(|b| *b == b'\n') {
        if line.is_empty() {
            break;
        }

        // TODO: is there some struct we can use (maybe in grep-printer) instead of Value?
        let json: Value = serde_json::from_slice(line).expect("json invalid");

        let match_type = json["type"].as_str().unwrap();
        match match_type {
            "begin" => {
                println!(
                    "new file: {}",
                    json["data"]["path"]["text"].as_str().unwrap()
                )
            }

            "match" => {
                println!(
                    "match (@{}): {}",
                    json["data"]["line_number"].as_u64().unwrap(),
                    json["data"]["lines"]["text"].as_str().unwrap().trim()
                );
            }

            _ => {
                println!("unhandled type: {}", match_type);
            }
        }
    }
}
