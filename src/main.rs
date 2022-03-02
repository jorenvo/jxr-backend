#[macro_use]
extern crate rocket;
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

#[get("/search?<query>")]
fn search(query: &str) -> String {
    let grep_json = run_ripgrep(
        &["/Users/jvo/Code/jxr-backend", "/Users/jvo/Code/gb-emu"],
        query,
    );

    String::from_utf8(grep_json).expect("rg did not return valid utf8")
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![search])
}
