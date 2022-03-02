use std::process::Command;

fn run_ripgrep(pattern: &str) -> String {
    let output = Command::new("rg")
        .arg("--json")
        .arg(pattern)
        .arg("/home/jvo/odoo_versions/odoo_v14")
        .arg("/home/jvo/odoo_versions/enterprise_v14")
        .output()
        .expect("failed to execute process");
    return String::from_utf8(output.stdout).expect("ripgrep output is not utf8");
}

fn main() {
    println!("Hello, world!");

    println!("{}", run_ripgrep("Please add your AvaTax credentials"));
}
