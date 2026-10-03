use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().canonicalize().unwrap().join(format!(
            "prefix-layer-evidence-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn empty_stderr_is_valid_but_required_evidence_is_not_empty() {
    let t = Temp::new();
    let mut e = Evidence::create(&t.0).unwrap();
    let pin = e.append("baseline-stderr.bin", &[], 2 << 20).unwrap();
    assert_eq!(pin.bytes, 0);
    assert_eq!(pin.sha256, hash(&[]));
    assert!(e.append("capture.bin", &[], 1).is_err());
    assert!(e.append("other-stderr.bin", &[], 1).is_err());
    assert!(e.json("baseline-stderr.bin", &1, 20).is_err());
    assert!(Evidence::create(&t.0).is_err());
}
#[test]
fn all_captures_and_maximum_other_parts_fit_distinct_evidence_envelope() {
    let t = Temp::new();
    let mut e = Evidence::create(&t.0).unwrap();
    e.append("request.json", &vec![b' '; 16384], 16384).unwrap();
    for label in ["baseline", "candidate"] {
        for name in ["registration", "program", "uploads"] {
            e.append(
                &format!("{label}-{name}.json"),
                &vec![b' '; 4 << 20],
                4 << 20,
            )
            .unwrap();
        }
        for name in [
            "bootstrap",
            "request-1",
            "response-1",
            "request-2",
            "response-2",
        ] {
            e.append(&format!("{label}-{name}.json"), &vec![b' '; 65536], 65536)
                .unwrap();
        }
        e.append(
            &format!("{label}-capture.bin"),
            &vec![0; super::super::wire::CAPTURE_BYTES],
            super::super::wire::CAPTURE_BYTES,
        )
        .unwrap();
        e.append(&format!("{label}-stderr.bin"), &vec![0; 2 << 20], 2 << 20)
            .unwrap();
    }
    e.append("parity.json", &vec![b' '; 65536], 65536).unwrap();
    assert_eq!(e.files.len(), 22);
    assert!(e.bytes + SUMMARY_RESERVE < LIMIT);
    e.finish(&serde_json::json!({"test_only":true})).unwrap();
}
#[test]
fn cumulative_entry_and_mutated_file_refusals_prevent_summary() {
    let t = Temp::new();
    let mut e = Evidence::create(&t.0).unwrap();
    assert!(e.append("../bad", &[1], 1).is_err());
    assert!(e.finish(&0).is_err());
    for i in 0..22 {
        e.append(&format!("part-{i}"), &[1], 1).unwrap();
    }
    fs::write(t.0.join("part-0"), [2]).unwrap();
    assert!(e.finish(&0).is_err());
    assert!(!t.0.join("summary.json").exists());
    e.bytes = LIMIT - SUMMARY_RESERVE;
    assert!(e.append("overflow", &[1], 1).is_err());
    e.bytes = 0;
    e.files.push(File {
        name: "extra".into(),
        bytes: 1,
        sha256: hash(&[1]),
    });
    assert!(e.append("last", &[1], 1).is_err());
}
