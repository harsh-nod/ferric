#![forbid(unsafe_code)]
use std::io::Write;

fn main() {
    let result = (|| {
        let mut args = std::env::args_os().skip(1);
        let root = args.next().ok_or("expected exactly one image directory")?;
        if args.next().is_some() {
            return Err("expected exactly one image directory".to_owned());
        }
        let raw = ferric_owned_kernel_admission_bench_v1::run(std::path::Path::new(&root))?;
        std::io::stdout()
            .lock()
            .write_all(&raw)
            .map_err(|e| e.to_string())
    })();
    if let Err(error) = result {
        eprintln!("admission benchmark refused: {error}");
        std::process::exit(1);
    }
}
