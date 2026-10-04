//! Separate raw KFD clock observations, without calibration or overlap claims.
use ferric_m1_engineering_execution_v1::tp_finite_client::prefix_decode::device_clock_v2::{
    self, Request,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

fn options(args: impl Iterator<Item = String>) -> Result<PathBuf, String> {
    let values = args.collect::<Vec<_>>();
    if values.len() != 4
        || values[0] != "--request"
        || values[2] != "--allow-unauthenticated-machine-code"
        || values[3] != "--observe-device-clocks"
    {
        return Err("exact --request FILE --allow-unauthenticated-machine-code --observe-device-clocks required".into());
    }
    let path = PathBuf::from(&values[1]);
    if !path.is_absolute() {
        return Err("absolute request required".into());
    }
    Ok(path)
}
fn execute() -> Result<(), String> {
    let path = options(std::env::args().skip(1))?;
    if path.canonicalize().map_err(|error| error.to_string())? != path
        || !std::fs::symlink_metadata(&path)
            .map_err(|error| error.to_string())?
            .is_file()
    {
        return Err("canonical regular request required".into());
    }
    let mut raw = Vec::new();
    std::fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(65537)
        .read_to_end(&mut raw)
        .map_err(|error| error.to_string())?;
    let value = device_clock_v2::run(Request::parse(&raw)?, true)?;
    let mut output = serde_json::to_vec(&value).map_err(|error| error.to_string())?;
    output.push(b'\n');
    if output.len() > 65536 {
        return Err("device clock diagnostic stdout bound".into());
    }
    std::io::stdout()
        .lock()
        .write_all(&output)
        .map_err(|error| error.to_string())
}
fn main() {
    if let Err(error) = execute() {
        eprintln!("finite prefix decode device clock diagnostic failure: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prefix_device_clock_bin_requires_its_distinct_optin_and_exact_arguments() {
        let good = [
            "--request",
            "/tmp/request.json",
            "--allow-unauthenticated-machine-code",
            "--observe-device-clocks",
        ];
        assert_eq!(
            options(good.iter().map(|value| value.to_string())).unwrap(),
            PathBuf::from(good[1])
        );
        for count in 0..4 {
            assert!(options(good[..count].iter().map(|value| value.to_string())).is_err());
        }
        for flag in [
            "--observe-device-ticks",
            "--observe-host-policy",
            "--observe-host",
            "--profile",
            "--raw-timestamps",
        ] {
            let mut wrong = good;
            wrong[3] = flag;
            assert!(options(wrong.iter().map(|value| value.to_string())).is_err());
            let mut extra = good.to_vec();
            extra.push(flag);
            assert!(options(extra.iter().map(|value| value.to_string())).is_err());
        }
        let mut wrong = good;
        wrong[1] = "relative";
        assert!(options(wrong.iter().map(|value| value.to_string())).is_err());
    }
}
