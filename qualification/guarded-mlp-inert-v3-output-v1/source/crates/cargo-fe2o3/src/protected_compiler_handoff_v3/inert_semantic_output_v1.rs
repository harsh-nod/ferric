//! Retained private output objects and bounded inert bytes, not producer authority.
#![allow(dead_code)] // The extraction selector remains held pending owned child integration.

use std::fs::{File, Metadata};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::FileExt;
use std::os::unix::fs::MetadataExt;

use fe2o3_compiler_ffi::{
    InertSemanticCompilerModuleHandoffIdentityV3,
    MAX_INERT_SEMANTIC_COMPILER_MODULE_HANDOFF_BYTES_V3,
};
use rustix::fs::{Mode, OFlags};

use crate::project::PinnedDirectory;

pub(crate) const OUTPUT_NAME: &str = "inert-semantic-handoff-v3.bin";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Snapshot {
    dev: u64,
    ino: u64,
    len: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    nlink: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}

impl Snapshot {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            len: m.len(),
            mode: m.mode(),
            uid: m.uid(),
            gid: m.gid(),
            nlink: m.nlink(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
        }
    }
}

pub(crate) struct PinnedOutput<'a> {
    directory: &'a PinnedDirectory,
    file: File,
    snapshot: Snapshot,
    bound: u64,
}

pub(crate) struct CapturedOutput<'a> {
    pinned: PinnedOutput<'a>,
    bytes: Box<[u8]>,
    expected: InertSemanticCompilerModuleHandoffIdentityV3,
}

impl<'a> PinnedOutput<'a> {
    pub(crate) fn open(
        directory: &'a PinnedDirectory,
        name: &str,
        bound: u64,
    ) -> Result<Self, String> {
        if name != OUTPUT_NAME
            || bound == 0
            || bound > MAX_INERT_SEMANTIC_COMPILER_MODULE_HANDOFF_BYTES_V3 as u64
        {
            return Err("inert output requires the fixed basename and bounded size".into());
        }
        validate_directory(directory)?;
        let file = open_slot(directory)?;
        let m = file.metadata().map_err(|e| e.to_string())?;
        validate_regular_private(&m, bound)?;
        let output = Self {
            directory,
            file,
            snapshot: Snapshot::of(&m),
            bound,
        };
        output.revalidate()?;
        Ok(output)
    }

    pub(crate) fn revalidate(&self) -> Result<(), String> {
        validate_directory(self.directory)?;
        let held = self.file.metadata().map_err(|e| e.to_string())?;
        validate_regular_private(&held, self.bound)?;
        let slot = open_slot(self.directory)?
            .metadata()
            .map_err(|e| e.to_string())?;
        if Snapshot::of(&held) != self.snapshot || Snapshot::of(&slot) != self.snapshot {
            return Err("inert output file changed or its directory entry was replaced".into());
        }
        validate_directory(self.directory)
    }

    pub(crate) fn capture(
        mut self,
        expected: InertSemanticCompilerModuleHandoffIdentityV3,
    ) -> Result<CapturedOutput<'a>, String> {
        let bytes = self.read_bytes()?;
        self.finish_capture(bytes, expected)
    }

    /// Metadata timestamps alone can miss same-length rewrites. Compare the
    /// retained object with the captured bytes using bounded positioned reads,
    /// then repeat object/path metadata checks. This is not writer exclusion.
    pub(crate) fn revalidate_bytes(&self, bytes: &[u8]) -> Result<(), String> {
        self.revalidate()?;
        if bytes.len() as u64 != self.snapshot.len {
            return Err("inert output byte length changed".into());
        }
        let mut buffer = [0u8; 64 * 1024];
        let mut offset = 0u64;
        for expected in bytes.chunks(buffer.len()) {
            self.file
                .read_exact_at(&mut buffer[..expected.len()], offset)
                .map_err(|e| e.to_string())?;
            if &buffer[..expected.len()] != expected {
                return Err("inert output bytes changed after capture".into());
            }
            offset += expected.len() as u64;
        }
        if self
            .file
            .read_at(&mut buffer[..1], offset)
            .map_err(|e| e.to_string())?
            != 0
        {
            return Err("inert output grew after capture".into());
        }
        self.revalidate()
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, String> {
        self.revalidate()?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.snapshot.len as usize)
            .map_err(|_| "inert output allocation failed")?;
        Read::by_ref(&mut self.file)
            .take(self.snapshot.len + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 != self.snapshot.len || bytes.len() as u64 > self.bound {
            return Err("inert output changed length during bounded read".into());
        }
        Ok(bytes)
    }

    fn finish_capture(
        self,
        bytes: Vec<u8>,
        expected: InertSemanticCompilerModuleHandoffIdentityV3,
    ) -> Result<CapturedOutput<'a>, String> {
        self.revalidate()?;
        if !expected.matches_canonical_bytes(&bytes) {
            return Err(
                "inert output differs from the independently expected complete outer identity"
                    .into(),
            );
        }
        self.revalidate_bytes(&bytes)?;
        Ok(CapturedOutput {
            pinned: self,
            bytes: bytes.into_boxed_slice(),
            expected,
        })
    }
}

impl<'a> CapturedOutput<'a> {
    pub(super) fn into_retained_parts(
        self,
    ) -> Result<
        (
            PinnedOutput<'a>,
            Box<[u8]>,
            InertSemanticCompilerModuleHandoffIdentityV3,
        ),
        String,
    > {
        self.pinned.revalidate_bytes(&self.bytes)?;
        Ok((self.pinned, self.bytes, self.expected))
    }
}

fn open_slot(directory: &PinnedDirectory) -> Result<File, String> {
    rustix::fs::openat(
        directory.file(),
        OUTPUT_NAME,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map(File::from)
    .map_err(|e| format!("cannot open inert output slot without following links: {e}"))
}

fn validate_directory(directory: &PinnedDirectory) -> Result<(), String> {
    let path = directory.display_path().as_os_str().as_bytes();
    if !path.starts_with(b"/")
        || path.len() < 2
        || path[1..]
            .split(|b| *b == b'/')
            .any(|s| s.is_empty() || s == b"." || s == b"..")
    {
        return Err("inert output directory must have an absolute alias-free pathname".into());
    }
    directory.validate_path("inert output directory")?;
    let m = directory.file().metadata().map_err(|e| e.to_string())?;
    if !m.is_dir() || m.uid() != rustix::process::geteuid().as_raw() || m.mode() & 0o777 != 0o700 {
        return Err(
            "inert output directory must be retained, user-owned and private mode0700".into(),
        );
    }
    Ok(())
}

fn validate_regular_private(m: &Metadata, bound: u64) -> Result<(), String> {
    if !m.is_file()
        || m.uid() != rustix::process::geteuid().as_raw()
        || m.mode() & 0o177 != 0
        || m.nlink() != 1
        || m.len() == 0
        || m.len() > bound
    {
        return Err(
            "inert output must be one nonempty bounded private non-executable ordinary file".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::inert_semantic_completion_v1::tests::{assert_retained_wire, baseline};
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink};
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> (
        std::path::PathBuf,
        PinnedDirectory,
        fe2o3_compiler_ffi::InertSemanticCompilerModuleHandoffV3,
    ) {
        let p = std::env::temp_dir().join(format!(
            "ferric-inert-output-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        let d = PinnedDirectory::open_existing(p.clone(), "fixture").unwrap();
        let wire = baseline();
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(p.join(OUTPUT_NAME))
            .unwrap();
        std::io::Write::write_all(&mut file, wire.canonical_bytes()).unwrap();
        (p, d, wire)
    }

    #[test]
    fn private_file_capture_retains_original_objects_and_full_codec_content() {
        let (_, d, wire) = fixture();
        let captured = PinnedOutput::open(&d, OUTPUT_NAME, wire.canonical_bytes().len() as u64)
            .unwrap()
            .capture(wire.identity())
            .unwrap();
        let (pinned, bytes, expected) = captured.into_retained_parts().unwrap();
        pinned.revalidate().unwrap();
        assert_eq!(&*bytes, wire.canonical_bytes());
        assert_eq!(expected, wire.identity());
        assert_retained_wire(&bytes, &wire);
    }

    #[test]
    fn fixed_slot_rejects_alias_names_and_unsupported_bounds_before_open() {
        let (_, d, wire) = fixture();
        for name in [
            "../inert-semantic-handoff-v3.bin",
            "./inert-semantic-handoff-v3.bin",
            "/inert-semantic-handoff-v3.bin",
            "",
        ] {
            assert!(PinnedOutput::open(&d, name, 10000).is_err());
        }
        assert!(PinnedOutput::open(&d, OUTPUT_NAME, 0).is_err());
        assert!(
            PinnedOutput::open(
                &d,
                OUTPUT_NAME,
                MAX_INERT_SEMANTIC_COMPILER_MODULE_HANDOFF_BYTES_V3 as u64 + 1
            )
            .is_err()
        );
        assert!(
            PinnedOutput::open(&d, OUTPUT_NAME, wire.canonical_bytes().len() as u64 - 1).is_err()
        );
    }

    #[test]
    fn symlink_hardlink_and_nonregular_slots_are_refused_without_blocking() {
        let (p, d, _) = fixture();
        fs::rename(p.join(OUTPUT_NAME), p.join("saved")).unwrap();
        symlink("saved", p.join(OUTPUT_NAME)).unwrap();
        assert!(PinnedOutput::open(&d, OUTPUT_NAME, 10000).is_err());
        fs::remove_file(p.join(OUTPUT_NAME)).unwrap();
        fs::hard_link(p.join("saved"), p.join(OUTPUT_NAME)).unwrap();
        assert!(PinnedOutput::open(&d, OUTPUT_NAME, 10000).is_err());
        fs::remove_file(p.join(OUTPUT_NAME)).unwrap();
        fs::create_dir(p.join(OUTPUT_NAME)).unwrap();
        assert!(PinnedOutput::open(&d, OUTPUT_NAME, 10000).is_err());
        fs::remove_dir(p.join(OUTPUT_NAME)).unwrap();
        rustix::fs::mkfifoat(d.file(), OUTPUT_NAME, Mode::RUSR | Mode::WUSR).unwrap();
        assert!(PinnedOutput::open(&d, OUTPUT_NAME, 10000).is_err());
    }

    #[test]
    fn replaced_file_entry_is_not_adopted_even_if_bytes_are_identical() {
        let (p, d, wire) = fixture();
        let pinned = PinnedOutput::open(&d, OUTPUT_NAME, 10000).unwrap();
        fs::rename(p.join(OUTPUT_NAME), p.join("saved")).unwrap();
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(p.join(OUTPUT_NAME))
            .unwrap();
        std::io::Write::write_all(&mut f, wire.canonical_bytes()).unwrap();
        assert!(pinned.capture(wire.identity()).is_err());
    }

    #[test]
    fn substituted_directory_path_cannot_redirect_retained_directory_reads() {
        let (p, d, wire) = fixture();
        let saved = p.with_extension("saved");
        fs::rename(&p, &saved).unwrap();
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(PinnedOutput::open(&d, OUTPUT_NAME, 10000).is_err());
        assert_eq!(
            fs::read(saved.join(OUTPUT_NAME)).unwrap(),
            wire.canonical_bytes()
        );
    }

    #[test]
    fn truncate_grow_same_length_write_and_permission_change_fail_postread_check() {
        for change in 0..4 {
            let (p, d, wire) = fixture();
            let mut pinned = PinnedOutput::open(&d, OUTPUT_NAME, 10000).unwrap();
            let bytes = pinned.read_bytes().unwrap();
            match change {
                0 => OpenOptions::new()
                    .write(true)
                    .open(p.join(OUTPUT_NAME))
                    .unwrap()
                    .set_len(1)
                    .unwrap(),
                1 => {
                    let mut f = OpenOptions::new()
                        .append(true)
                        .open(p.join(OUTPUT_NAME))
                        .unwrap();
                    std::io::Write::write_all(&mut f, b"x").unwrap();
                }
                2 => {
                    let mut changed = wire.canonical_bytes().to_vec();
                    changed[0] ^= 1;
                    fs::write(p.join(OUTPUT_NAME), changed).unwrap();
                }
                _ => fs::set_permissions(p.join(OUTPUT_NAME), fs::Permissions::from_mode(0o400))
                    .unwrap(),
            }
            assert!(
                pinned.finish_capture(bytes, wire.identity()).is_err(),
                "change {change}"
            );
        }
    }

    #[test]
    fn empty_public_file_public_directory_and_changed_identity_are_refused() {
        let (p, d, wire) = fixture();
        fs::set_permissions(p.join(OUTPUT_NAME), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(PinnedOutput::open(&d, OUTPUT_NAME, 10000).is_err());
        fs::set_permissions(p.join(OUTPUT_NAME), fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(PinnedOutput::open(&d, OUTPUT_NAME, 10000).is_err());
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        let mut altered = wire.canonical_bytes().to_vec();
        altered[0] ^= 1;
        fs::write(p.join(OUTPUT_NAME), altered).unwrap();
        assert!(
            PinnedOutput::open(&d, OUTPUT_NAME, 10000)
                .unwrap()
                .capture(wire.identity())
                .is_err()
        );
        fs::write(p.join(OUTPUT_NAME), b"").unwrap();
        assert!(PinnedOutput::open(&d, OUTPUT_NAME, 10000).is_err());
    }

    #[test]
    fn captured_file_is_revalidated_before_transfer_and_alias_directory_is_refused() {
        let (p, d, wire) = fixture();
        let captured = PinnedOutput::open(&d, OUTPUT_NAME, 10000)
            .unwrap()
            .capture(wire.identity())
            .unwrap();
        fs::write(p.join(OUTPUT_NAME), b"changed").unwrap();
        assert!(captured.into_retained_parts().is_err());
        let (p, d, wire) = fixture();
        let captured = PinnedOutput::open(&d, OUTPUT_NAME, 10000)
            .unwrap()
            .capture(wire.identity())
            .unwrap();
        let mut changed = wire.canonical_bytes().to_vec();
        changed[0] ^= 1;
        fs::write(p.join(OUTPUT_NAME), changed).unwrap();
        assert!(captured.into_retained_parts().is_err());
        let aliased = std::path::PathBuf::from(format!("{}/./", p.display()));
        let alias = PinnedDirectory::open_existing(aliased, "alias fixture").unwrap();
        assert!(PinnedOutput::open(&alias, OUTPUT_NAME, 10000).is_err());
    }
}
