use std::env;
use std::process::Command;

fn main() {
    match real_main() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("xtask: {err}");
            std::process::exit(1);
        }
    }
}

fn real_main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("quick") => quick(),
        Some("changed") => changed(),
        Some("full") => full(),
        Some("inventory") => inventory(args.collect()),
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unsupported command `{other}`")),
        None => {
            print_help();
            Err("missing command".to_string())
        }
    }
}

fn quick() -> Result<(), String> {
    run("cargo", &["test", "-p", "avutil", "-p", "fftools"])
}

fn changed() -> Result<(), String> {
    quick()
}

fn full() -> Result<(), String> {
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("cargo", &["test", "--workspace", "--all-features"])
}

fn inventory(args: Vec<String>) -> Result<(), String> {
    let mut command_args = vec!["run", "-p", "oracle", "--", "inventory"];
    let owned: Vec<String> = args;
    for arg in &owned {
        command_args.push(arg);
    }
    run("cargo", &command_args)
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|err| format!("failed to run {program}: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{program} {}` exited with status {status}",
            args.join(" ")
        ))
    }
}

fn print_help() {
    eprintln!("usage: cargo run -p xtask -- <quick|changed|full|inventory>");
}
