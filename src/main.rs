use std::process::Command;

fn run_ripgrep(paths: &[&str], pattern: &str) -> String {
    let mut command = Command::new("rg");

    command.arg("--json");
    command.arg(pattern);

    for path in paths {
        command.arg(path);
    }

    let output = command.output().expect("failed to execute process");
    String::from_utf8(output.stdout).expect("ripgrep output is not utf8")
}

fn main() {
    println!("Hello, world!");
    println!(
        "{}",
        run_ripgrep(
            &["/Users/jvo/Code/jxr-backend", "/Users/jvo/Code/gb-emu"],
            "ripgrep output is not utf8"
        )
    );
}
