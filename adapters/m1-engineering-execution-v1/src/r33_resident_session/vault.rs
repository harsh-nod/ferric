//! Private, non-authoritative storage for resident transition owners.

use core::mem::ManuallyDrop;
use std::path::PathBuf;

use ferric_spec::TokenId;

pub(super) const EXTERNAL_DROP_MARKER_ENV_V1: &str =
    "FERRIC_R33_CUSTODY_VAULT_EXTERNAL_DROP_MARKER_V1";

/// Both transition-sensitive owners remain manually retained until a vault
/// method selects an exact normal release path. Any future lower physical API
/// that consumes owners by value must be implemented inside this child module
/// and remain bracketed by `AbortOnUnwind`; it must not widen the capability.
pub(super) struct Vault<C, I> {
    custody: Option<ManuallyDrop<C>>,
    input: Option<ManuallyDrop<I>>,
    entered_windows: u64,
}

impl<C, I> Vault<C, I> {
    pub(super) fn new(custody: C) -> Self {
        Self {
            custody: Some(ManuallyDrop::new(custody)),
            input: None,
            entered_windows: 0,
        }
    }

    pub(super) fn install_input(&mut self, input: I) -> Result<(), I> {
        if self.input.is_some() {
            return Err(input);
        }
        self.input = Some(ManuallyDrop::new(input));
        Ok(())
    }

    pub(super) fn execution_capability<'a>(
        &'a mut self,
        view: WindowView<'a>,
    ) -> Option<ExecutionCapability<'a, C, I>> {
        if self.custody.is_none() || self.input.is_none() {
            return None;
        }
        self.entered_windows = self.entered_windows.checked_add(1)?;
        Some(ExecutionCapability { vault: self, view })
    }

    pub(super) fn complete_input(&mut self) -> bool {
        let Some(input) = self.input.take() else {
            return false;
        };
        drop(ManuallyDrop::into_inner(input));
        true
    }

    pub(super) fn recover_input(&mut self) -> Option<I> {
        self.input.take().map(ManuallyDrop::into_inner)
    }

    pub(super) fn recover_custody(&mut self) -> Option<C> {
        if self.input.is_some() {
            return None;
        }
        self.custody.take().map(ManuallyDrop::into_inner)
    }

    pub(super) fn release_all(&mut self) -> bool {
        let Some(custody) = self.custody.take() else {
            return false;
        };
        if let Some(input) = self.input.take() {
            drop(ManuallyDrop::into_inner(input));
        }
        drop(ManuallyDrop::into_inner(custody));
        true
    }

    #[allow(clippy::forget_non_drop)]
    pub(super) fn quarantine_input(&mut self) {
        // Dropping the wrapper never drops its manually retained inner owner.
        let _quarantined_input = self.input.take();
    }

    #[allow(clippy::forget_non_drop)]
    pub(super) fn quarantine_all(&mut self) {
        // Dropping either wrapper never drops its manually retained inner owner.
        let _quarantined_input = self.input.take();
        let _quarantined_custody = self.custody.take();
    }
}

impl<C, I> Drop for Vault<C, I> {
    fn drop(&mut self) {
        self.quarantine_all();
    }
}

/// Immutable window facts exposed through a non-owning execution capability.
pub(super) struct WindowView<'a> {
    pub(super) sequence: usize,
    pub(super) row_ordinal: u64,
    pub(super) prompt_tokens: &'a [TokenId],
    pub(super) expected_output_tokens: u64,
}

/// The only value visible to executor implementations.
///
/// Its fields are private to this child module. No method returns `C`, `I`,
/// `&mut C`, or `&mut I`, so safe executor code cannot drop, replace, or
/// substitute either owner.
pub(super) struct ExecutionCapability<'a, C, I> {
    vault: &'a mut Vault<C, I>,
    view: WindowView<'a>,
}

impl<C, I> ExecutionCapability<'_, C, I> {
    pub(super) const fn sequence(&self) -> usize {
        self.view.sequence
    }

    pub(super) const fn row_ordinal(&self) -> u64 {
        self.view.row_ordinal
    }

    pub(super) fn prompt_tokens(&self) -> &[TokenId] {
        self.view.prompt_tokens
    }

    pub(super) const fn expected_output_tokens(&self) -> u64 {
        self.view.expected_output_tokens
    }

    pub(super) const fn entered_windows(&self) -> u64 {
        self.vault.entered_windows
    }
}

/// Armed immediately before execution code receives a capability. Any unwind
/// or early return that does not explicitly disarm terminates the process while
/// vault-owned `ManuallyDrop` values remain retained.
pub(super) struct AbortOnUnwind {
    armed: bool,
}

impl AbortOnUnwind {
    pub(super) const fn armed() -> Self {
        Self { armed: true }
    }

    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for AbortOnUnwind {
    fn drop(&mut self) {
        if self.armed {
            std::process::abort();
        }
    }
}

struct ExternalDropMarker {
    path: PathBuf,
    byte: u8,
}

impl Drop for ExternalDropMarker {
    fn drop(&mut self) {
        let _ = std::fs::write(&self.path, [self.byte]);
    }
}

/// Shared probe used by the built helper to prove exact abort behavior.
pub(super) fn run_external_abort_probe_v1() -> ! {
    let Some(marker_path) = std::env::var_os(EXTERNAL_DROP_MARKER_ENV_V1) else {
        eprintln!("missing {EXTERNAL_DROP_MARKER_ENV_V1}");
        std::process::exit(2);
    };
    let marker_path = PathBuf::from(marker_path);
    let mut vault = Vault::new(ExternalDropMarker {
        path: marker_path.clone(),
        byte: b'C',
    });
    if vault
        .install_input(ExternalDropMarker {
            path: marker_path,
            byte: b'I',
        })
        .is_err()
    {
        std::process::exit(3);
    }
    let _abort_on_unwind = AbortOnUnwind::armed();
    panic!("intentional custody-vault abort probe")
}
