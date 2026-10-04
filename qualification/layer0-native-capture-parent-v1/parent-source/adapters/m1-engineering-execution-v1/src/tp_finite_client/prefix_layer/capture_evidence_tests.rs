use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().canonicalize().unwrap().join(format!(
            "prefix-layer-capture-evidence-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        )))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn populate(e: &mut Evidence, names: &[&str]) {
    for &name in names {
        if name == "candidate-stderr.bin" {
            e.append(name, &[], 1).unwrap();
        } else {
            e.append(name, &[1], 1).unwrap();
        }
    }
}

#[test]
fn exact_candidate_roster_can_finalize_but_not_as_paired_evidence() {
    let t = Temp::new();
    let mut e = Evidence::create(&t.0).unwrap();
    populate(&mut e, &CAPTURE_FILES);
    assert!(e.finish(&0).is_err());
    assert!(!t.0.join("summary.json").exists());
    e.finish_capture(&serde_json::json!({"test_only":true}))
        .unwrap();
    assert!(t.0.join("summary.json").is_file());
    assert!(e.finish_capture(&0).is_err());
}

#[test]
fn candidate_roster_rejects_missing_extra_reordered_and_paired_names() {
    for which in 0..4 {
        let t = Temp::new();
        let mut e = Evidence::create(&t.0).unwrap();
        let mut names = CAPTURE_FILES.to_vec();
        match which {
            0 => {
                names.pop();
            }
            1 => names.push("parity.json"),
            2 => names.swap(5, 7),
            _ => names[1] = "baseline-registration.json",
        }
        populate(&mut e, &names);
        assert!(e.finish_capture(&0).is_err());
        assert!(!t.0.join("summary.json").exists());
    }
}

#[test]
fn candidate_summary_refuses_changed_deleted_and_symlinked_capture() {
    for which in 0..3 {
        let t = Temp::new();
        let mut e = Evidence::create(&t.0).unwrap();
        populate(&mut e, &CAPTURE_FILES);
        let path = t.0.join("candidate-capture.bin");
        match which {
            0 => fs::write(&path, [2]).unwrap(),
            1 => fs::remove_file(&path).unwrap(),
            _ => {
                fs::remove_file(&path).unwrap();
                std::os::unix::fs::symlink("request.json", &path).unwrap();
            }
        }
        assert!(e.finish_capture(&0).is_err());
        assert!(!t.0.join("summary.json").exists());
    }
}

#[test]
fn candidate_summary_retains_original_aggregate_and_summary_bounds() {
    let t = Temp::new();
    let mut e = Evidence::create(&t.0).unwrap();
    populate(&mut e, &CAPTURE_FILES);
    assert!(e.finish_capture(&"x".repeat(SUMMARY_RESERVE)).is_err());
    assert!(!t.0.join("summary.json").exists());
    e.bytes = LIMIT;
    assert!(e.finish_capture(&1).is_err());
    assert!(!t.0.join("summary.json").exists());
}
