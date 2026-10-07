//! `fasttype` : clone de Monkeytype pour le terminal.

use fasttype_store::Store;
use fasttype_store::paths::Paths;
use fasttype_tui::runner::{parse_options, run};
use std::process::ExitCode;

const USAGE: &str =
    "usage: fasttype [--perf] [--fps 60|120|144] | --rebuild-pbs | --version | --help

  --perf          show input latency and frame time
  --fps N         animation frames per second (default 60)
  --rebuild-pbs   recompute personal bests from the result history";

fn rebuild_pbs() -> ExitCode {
    let Some(paths) = Paths::from_system() else {
        eprintln!("HOME is not set");
        return ExitCode::FAILURE;
    };
    let mut store = Store::open(paths);
    match store.rebuild_pbs() {
        Ok(h) => {
            println!(
                "{} results read, {} unreadable lines skipped",
                h.results.len(),
                h.skipped_lines
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--rebuild-pbs") => return rebuild_pbs(),
        Some("--version" | "-V") => {
            println!("fasttype {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("--help" | "-h") => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    let opts = match parse_options(&args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let perf = opts.perf;
    match run(opts) {
        Ok(stats) => {
            if perf {
                println!("{}", stats.summary());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
