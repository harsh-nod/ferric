//! Built helper proving custody-vault abort and destructor behavior.

#[allow(dead_code)] // The probe deliberately uses only vault retention and abort.
#[path = "../r33_resident_session/vault.rs"]
mod vault;

fn main() {
    vault::run_external_abort_probe_v1();
}
