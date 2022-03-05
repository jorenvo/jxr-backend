#[macro_use]
extern crate rocket;
use std::process::Command;

fn run_ripgrep(pattern: &str) -> Vec<u8> {
    let mut command = Command::new("rg");

    // TODO: this should be configurable
    command.current_dir("/Users/jvo/Code/jxr-frontend/dist/jxr-code");

    command.arg("--json");
    command.arg(pattern);

    let output = command.output().expect("failed to execute process");
    output.stdout
}

// TODO: this won't work if newlines appear in the json (unlikely since we won't support
// multi-line search). There should be a nicer way to do this.
fn rg_sequence_to_array(json: &mut Vec<u8>) {
    // replace all newlines with commas (skip last newline to avoid trailing comma)
    for i in 0..json.len() - 1 {
        if json[i] == b'\n' {
            json[i] = b',';
        }
    }

    // enclose in [] so we end up with a json array
    json.insert(0, b'[');
    json.push(b']');
}

#[get("/search?<query>")]
fn search(query: &str) -> String {
    let mut grep_json = run_ripgrep(query);

    rg_sequence_to_array(&mut grep_json);

    println!("finished searching for {}", query);
    String::from_utf8(grep_json).expect("rg did not return valid utf8")
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![search])
}
