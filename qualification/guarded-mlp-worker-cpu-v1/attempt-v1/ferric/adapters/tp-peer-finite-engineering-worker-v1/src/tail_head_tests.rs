use super::*;

#[derive(Default)]
struct Fake {
    events: Vec<&'static str>,
    fail: Option<&'static str>,
    alias: bool,
}
impl Backend for Fake {
    type Buffer = u32;
    fn preflight(&mut self) -> Result<()> {
        self.events.push("preflight");
        if self.fail == Some("preflight") {
            Err("capacity".into())
        } else {
            Ok(())
        }
    }
    fn allocate(&mut self) -> Result<u32> {
        self.events.push("allocate");
        if self.fail == Some("allocate") {
            Err("allocation".into())
        } else {
            Ok(if self.alias { 1 } else { 2 })
        }
    }
    fn write(&mut self, _: u32, _: u64, _: &[u8]) -> Result<()> {
        self.events.push("write");
        if self.fail == Some("write") {
            Err("write".into())
        } else {
            Ok(())
        }
    }
}
fn manifest() -> HeadManifest {
    HeadManifest {
        source_id: 42,
        source_sha256: [1; 32],
        output_sha256: [2; 32],
    }
}
fn small() -> Upload<u32, 8> {
    Upload {
        buffer: 2,
        manifest: HeadManifest {
            output_sha256: Sha256::digest([1u8; 8]).into(),
            ..manifest()
        },
        offset: 0,
        hasher: Sha256::new(),
        terminal: false,
        complete: false,
    }
}

#[test]
fn transpose_manifest_join_precedes_preflight_and_allocation() {
    for bad in 0..4 {
        let mut m = manifest();
        match bad {
            0 => m.source_id = 0,
            1 => m.source_id = 41,
            2 => m.source_sha256 = [3; 32],
            _ => m.output_sha256 = [0; 32],
        }
        let mut fake = Fake::default();
        assert!(allocate(&mut fake, 1, 42, [1; 32], m).is_err());
        assert!(fake.events.is_empty());
    }
    let mut fake = Fake::default();
    let upload = allocate(&mut fake, 1, 42, [1; 32], manifest()).unwrap();
    assert_eq!(fake.events, ["preflight", "allocate"]);
    assert!(upload.buffer().is_err());
}

#[test]
fn transpose_capacity_allocation_and_original_alias_fail_closed() {
    for failure in ["preflight", "allocate"] {
        let mut fake = Fake {
            fail: Some(failure),
            ..Fake::default()
        };
        assert!(allocate(&mut fake, 1, 42, [1; 32], manifest()).is_err());
        assert_eq!(fake.events.last(), Some(&failure));
    }
    let mut fake = Fake {
        alias: true,
        ..Fake::default()
    };
    assert!(allocate(&mut fake, 1, 42, [1; 32], manifest()).is_err());
}

#[test]
fn transpose_only_complete_sequential_digest_exposes_token() {
    // Reduced private extent uses exactly the production upload state machine.
    let mut upload = small();
    let mut fake = Fake::default();
    upload.write(&mut fake, 0, &[1; 4]).unwrap();
    assert!(upload.buffer().is_err());
    upload.write(&mut fake, 4, &[1; 4]).unwrap();
    assert_eq!(upload.buffer().unwrap(), 2);
    assert!(upload.write(&mut fake, 8, &[1; 2]).is_err());
    assert!(upload.buffer().is_err());
    assert_eq!(fake.events, ["write", "write"]);
}

#[test]
fn transpose_bad_upload_or_native_error_is_terminal_and_never_retried() {
    let cases = [
        (2, vec![1; 2]),
        (0, vec![]),
        (0, vec![1; 3]),
        (0, vec![1; 10]),
        (u64::MAX, vec![1; 2]),
    ];
    for (offset, bytes) in cases {
        let mut upload = small();
        let mut fake = Fake::default();
        assert!(upload.write(&mut fake, offset, &bytes).is_err());
        assert!(upload.write(&mut fake, 0, &[1; 8]).is_err());
        assert!(fake.events.is_empty());
        assert!(upload.buffer().is_err());
    }
    let mut upload = small();
    let mut fake = Fake {
        fail: Some("write"),
        ..Fake::default()
    };
    assert!(upload.write(&mut fake, 0, &[1; 8]).is_err());
    fake.fail = None;
    assert!(upload.write(&mut fake, 0, &[1; 8]).is_err());
    assert_eq!(fake.events, ["write"]);
}

#[test]
fn transpose_digest_failure_never_exposes_completed_buffer() {
    let mut upload = small();
    let mut fake = Fake::default();
    assert!(upload.write(&mut fake, 0, &[2; 8]).is_err());
    assert_eq!(upload.offset, 8);
    assert!(upload.terminal);
    assert!(!upload.complete);
    assert!(upload.buffer().is_err());
}
