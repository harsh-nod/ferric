//! Separate fixed two-forward engineering command, not the queued runner.
use ferric_m1_engineering_execution_v1::tp_finite_client::{Config, run};
use std::io::Read;
use std::path::PathBuf;

fn options(arguments: impl Iterator<Item = String>) -> Result<PathBuf, String> {
    let mut arguments = arguments;
    let mut request = None;
    let mut consent = false;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--request" if request.is_none() => request = Some(PathBuf::from(arguments.next().ok_or("missing request path")?)),
            "--allow-unauthenticated-machine-code" if !consent => consent = true,
            _ => return Err("only --request FILE and explicit --allow-unauthenticated-machine-code are accepted".into()),
        }
    }
    if !consent {
        return Err("explicit engineering machine-code opt-in required".into());
    }
    let path = request.ok_or("--request is required")?;
    if !path.is_absolute() {
        return Err("request path must be absolute".into());
    }
    Ok(path)
}
fn execute() -> Result<(), String> {
    let path = options(std::env::args().skip(1))?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let observation = run(Config::parse(&bytes)?, true)?;
    // Publish only after both completed forwards, native close and process reap.
    serde_json::to_writer(std::io::stdout().lock(), &observation).map_err(|e| e.to_string())?;
    println!();
    Ok(())
}
fn main() {
    if let Err(error) = execute() {
        eprintln!("finite-two-forward engineering failure: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(values: &[&str]) -> Result<PathBuf, String> {
        options(values.iter().map(|v| (*v).to_owned()))
    }
    #[test]
    fn command_is_explicit_closed_and_distinct_from_queued_options() {
        assert_eq!(
            parse(&[
                "--request",
                "/task/request.json",
                "--allow-unauthenticated-machine-code"
            ])
            .unwrap(),
            PathBuf::from("/task/request.json")
        );
        for values in [
            vec!["--request", "/task/request.json"],
            vec!["--allow-unauthenticated-machine-code"],
            vec![
                "--request",
                "relative",
                "--allow-unauthenticated-machine-code",
            ],
            vec![
                "--request",
                "/a",
                "--request",
                "/b",
                "--allow-unauthenticated-machine-code",
            ],
            vec![
                "--request",
                "/a",
                "--allow-unauthenticated-machine-code",
                "--runtime-tp2-queued-graph",
                "baseline",
            ],
        ] {
            assert!(parse(&values).is_err());
        }
    }
}
