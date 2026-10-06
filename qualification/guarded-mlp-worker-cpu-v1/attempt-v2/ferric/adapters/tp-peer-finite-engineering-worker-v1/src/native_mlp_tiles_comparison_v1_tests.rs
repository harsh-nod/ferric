use super::super::{MetadataWriter, write_metadata};
use super::*;
fn input() -> ForwardInput {
    ForwardInput {
        registration: [1; 32],
        generation: 1,
        token: TOKEN,
        cache_metadata: std::array::from_fn(|i| if i == 0 { 0 } else { (i - 1) as u32 }),
        rotary_bits: [0; 128],
    }
}
#[test]
fn only_fresh_generation_one_position_zero_token_can_activate_pair() {
    for mutation in 0..3 {
        let mut value = input();
        match mutation {
            0 => value.generation = 2,
            1 => value.cache_metadata[0] = 1,
            _ => value.token += 1,
        }
        let mut gate = Gate::Fresh;
        assert!(gate.begin(&value).is_err());
        assert_eq!(gate, Gate::Failed);
        assert!(gate.commit().is_err());
    }
    let mut gate = Gate::Fresh;
    gate.begin(&input()).unwrap();
    gate.commit().unwrap();
    assert_eq!(gate, Gate::Completed);
    assert!(gate.begin(&input()).is_err());
    assert_eq!(gate, Gate::Failed);
}
#[test]
fn duplicate_activation_or_commit_without_whole_forward_is_terminal() {
    let mut gate = Gate::Fresh;
    gate.begin(&input()).unwrap();
    assert!(gate.begin(&input()).is_err());
    assert_eq!(gate, Gate::Failed);
    for phase in [Gate::Fresh, Gate::Completed, Gate::Failed] {
        let mut gate = phase;
        assert!(gate.commit().is_err());
        assert_eq!(gate, Gate::Failed);
    }
}
#[test]
fn frozen_scope_fields_are_joined_without_refusing_valid_zero_group() {
    let expected = Scope {
        bundle_id: [1; 32],
        model_id: [2; 32],
        session: [3; 32],
        pool_identity: 4,
        group_id: 0,
        child_identity: 5,
    };
    let actual = SourceScope {
        bundle_id: expected.bundle_id,
        model_id: expected.model_id,
        session: expected.session,
        pool_identity: expected.pool_identity,
        group_id: 0,
        child_identity: 5,
    };
    require_scope(&actual, &expected).unwrap();
    for i in 0..6 {
        let mut bad = actual.clone();
        match i {
            0 => bad.bundle_id[0] ^= 1,
            1 => bad.model_id[0] ^= 1,
            2 => bad.session[0] ^= 1,
            3 => bad.pool_identity += 1,
            4 => bad.group_id += 1,
            _ => bad.child_identity += 1,
        }
        assert!(require_scope(&bad, &expected).is_err());
    }
}
#[test]
fn old_and_new_allocation_profiles_cannot_substitute_for_each_other() {
    for profile in [
        AllocationProfile::BaseV1,
        AllocationProfile::TilesComparisonV1,
    ] {
        profile.validate(&profile.counts()).unwrap();
        for bad in [
            vec![],
            vec![715],
            vec![714, 710, 0],
            vec![714, 711],
            vec![715, 710],
            vec![716, 712],
        ] {
            assert!(profile.validate(&bad).is_err());
        }
    }
    assert!(AllocationProfile::BaseV1.validate(&[715, 711]).is_err());
    assert!(
        AllocationProfile::TilesComparisonV1
            .validate(&[714, 710])
            .is_err()
    );
}

#[test]
fn actual_metadata_writer_checks_selected_count_before_any_write_and_stops_on_error() {
    struct Writer {
        counts: Vec<usize>,
        events: Vec<usize>,
        fail: Option<usize>,
    }
    impl MetadataWriter for Writer {
        type Root = usize;
        fn counts(&mut self) -> Result<Vec<usize>> {
            self.events.push(99);
            if self.fail == Some(0) {
                Err("fence".into())
            } else {
                Ok(self.counts.clone())
            }
        }
        fn write_metadata_root(&mut self, root: usize, bytes: &[u8]) -> Result<()> {
            assert_eq!(
                bytes,
                if root % 2 == 0 {
                    b"metadata".as_slice()
                } else {
                    b"rotary".as_slice()
                }
            );
            let index = self.events.len();
            self.events.push(root);
            if self.fail == Some(index) {
                Err("write".into())
            } else {
                Ok(())
            }
        }
    }
    for profile in [
        AllocationProfile::BaseV1,
        AllocationProfile::TilesComparisonV1,
    ] {
        let other = if profile == AllocationProfile::BaseV1 {
            [715, 711]
        } else {
            [714, 710]
        };
        let mut writer = Writer {
            counts: other.to_vec(),
            events: vec![],
            fail: None,
        };
        assert!(
            write_metadata(&mut writer, profile, [0, 1, 2, 3], b"metadata", b"rotary").is_err()
        );
        assert_eq!(writer.events, [99]);
        for fail in [None, Some(0), Some(1), Some(2), Some(3), Some(4)] {
            let mut writer = Writer {
                counts: profile.counts().to_vec(),
                events: vec![],
                fail,
            };
            let result = write_metadata(&mut writer, profile, [0, 1, 2, 3], b"metadata", b"rotary");
            assert_eq!(result.is_ok(), fail.is_none());
            assert_eq!(writer.events.len(), fail.map_or(5, |i| i + 1));
        }
    }
}
