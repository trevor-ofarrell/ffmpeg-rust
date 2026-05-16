use std::env;
use std::fs;

fn main() {
    match real_main() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("fate-runner: {err}");
            std::process::exit(1);
        }
    }
}

fn real_main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("list") => list_components(),
        Some("run") => run_component(args.collect()),
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

fn list_components() -> Result<(), String> {
    let ledger = fs::read_to_string("PORTING_LEDGER.toml")
        .map_err(|err| format!("failed to read PORTING_LEDGER.toml: {err}"))?;
    let ids: Vec<&str> = ledger
        .lines()
        .filter_map(|line| line.trim().strip_prefix("id = "))
        .map(|value| value.trim_matches('"'))
        .collect();

    if ids.is_empty() {
        println!("no ledger components found");
    } else {
        for id in ids {
            println!("{id}");
        }
    }
    Ok(())
}

fn run_component(args: Vec<String>) -> Result<(), String> {
    if args.first().map(String::as_str) == Some("--changed") {
        return Err(
            "changed FATE selection is not mapped yet; ledger records this limitation".to_string(),
        );
    }

    let mut component = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--component" => component = iter.next().cloned(),
            other => return Err(format!("unsupported run argument `{other}`")),
        }
    }

    let component = component.ok_or_else(|| "missing --component <id>".to_string())?;
    Err(format!(
        "component `{component}` has no runnable FATE mapping in the current slice"
    ))
}

fn print_help() {
    eprintln!("usage: fate-runner list | run --component <id> | run --changed");
}
