use core::fmt;
use vstd::prelude::*;

verus! {

pub type TokenId = u32;

/// The observable result of one greedy speculative verification round.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreedyCommit {
    accepted_draft_tokens: usize,
    emitted_tokens: Vec<TokenId>,
    target_correction_or_bonus: TokenId,
}

impl GreedyCommit {
    pub closed spec fn accepted_spec(&self) -> nat {
        self.accepted_draft_tokens as nat
    }

    pub closed spec fn emitted_spec(&self) -> Seq<TokenId> {
        self.emitted_tokens@
    }

    pub closed spec fn correction_or_bonus_spec(&self) -> TokenId {
        self.target_correction_or_bonus
    }

    #[must_use]
    pub const fn accepted_draft_tokens(&self) -> (accepted: usize)
        ensures accepted as nat == self.accepted_spec(),
    {
        self.accepted_draft_tokens
    }

    #[must_use]
    pub fn emitted_tokens(&self) -> (emitted: &[TokenId])
        ensures emitted@ == self.emitted_spec(),
    {
        &self.emitted_tokens
    }

    #[must_use]
    pub const fn target_correction_or_bonus(&self) -> (token: TokenId)
        ensures token == self.correction_or_bonus_spec(),
    {
        self.target_correction_or_bonus
    }
}

#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GreedyVerificationError {
    MissingTargetBonus,
    DraftTokenCountOverflow,
    IncorrectTargetChoiceCount { expected: usize, actual: usize },
}

} // verus!

impl fmt::Display for GreedyVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTargetBonus => {
                formatter.write_str("a speculative round requires at least one target choice")
            }
            Self::DraftTokenCountOverflow => {
                formatter.write_str("draft token count cannot be represented with a bonus token")
            }
            Self::IncorrectTargetChoiceCount { expected, actual } => write!(
                formatter,
                "target choices must contain one choice per draft token plus one bonus: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for GreedyVerificationError {}

verus! {

pub open spec fn is_greedy_accepted_prefix(
    draft_tokens: Seq<TokenId>,
    target_choices: Seq<TokenId>,
    accepted: nat,
) -> bool {
    accepted <= draft_tokens.len()
        && accepted < target_choices.len()
        && forall|index: int|
            0 <= index < accepted
                ==> draft_tokens[index] == target_choices[index]
        && (accepted == draft_tokens.len()
            || draft_tokens[accepted as int] != target_choices[accepted as int])
}

pub open spec fn greedy_commit_matches(
    draft_tokens: Seq<TokenId>,
    target_choices: Seq<TokenId>,
    commit: GreedyCommit,
) -> bool {
    is_greedy_accepted_prefix(
        draft_tokens,
        target_choices,
        commit.accepted_spec(),
    )
        && commit.emitted_spec()
            == draft_tokens.subrange(0, commit.accepted_spec() as int).push(
                target_choices[commit.accepted_spec() as int],
            )
        && commit.correction_or_bonus_spec()
            == target_choices[commit.accepted_spec() as int]
}

/// Applies the canonical greedy speculative-decoding transition.
///
/// `target_choices[i]` is the target argmax after the already accepted prefix.
/// The final target choice is the bonus token used when every draft token is
/// accepted. At the first mismatch the target choice is emitted as the
/// correction and the remaining draft suffix is rejected.
///
/// # Errors
///
/// Returns [`GreedyVerificationError`] unless the target provides one choice
/// for every draft position and one final correction-or-bonus choice.
pub fn verify_greedy_round(
    draft_tokens: &[TokenId],
    target_choices: &[TokenId],
) -> (result: Result<GreedyCommit, GreedyVerificationError>)
    ensures
        match result {
            Err(GreedyVerificationError::MissingTargetBonus) => {
                target_choices@.len() == 0
            },
            Err(GreedyVerificationError::DraftTokenCountOverflow) => {
                target_choices@.len() > 0
                    && draft_tokens@.len() == usize::MAX
            },
            Err(GreedyVerificationError::IncorrectTargetChoiceCount {
                expected,
                actual,
            }) => {
                target_choices@.len() > 0
                    && expected as nat == draft_tokens@.len() + 1
                    && actual as nat == target_choices@.len()
                    && target_choices@.len() != draft_tokens@.len() + 1
            },
            Ok(commit) => {
                target_choices@.len() == draft_tokens@.len() + 1
                    && greedy_commit_matches(draft_tokens@, target_choices@, commit)
            },
        },
{
    if target_choices.is_empty() {
        return Err(GreedyVerificationError::MissingTargetBonus);
    }

    let Some(expected) = draft_tokens.len().checked_add(1) else {
        return Err(GreedyVerificationError::DraftTokenCountOverflow);
    };
    if target_choices.len() != expected {
        return Err(GreedyVerificationError::IncorrectTargetChoiceCount {
            expected,
            actual: target_choices.len(),
        });
    }

    let mut accepted = 0;
    while accepted < draft_tokens.len()
        && draft_tokens[accepted] == target_choices[accepted]
        invariant
            target_choices@.len() == draft_tokens@.len() + 1,
            accepted <= draft_tokens@.len(),
            accepted < target_choices@.len(),
            forall|index: int|
                0 <= index < accepted
                    ==> draft_tokens@[index] == target_choices@[index],
        decreases draft_tokens@.len() - accepted,
    {
        accepted += 1;
    }

    let correction_or_bonus = target_choices[accepted];
    let mut emitted = Vec::new();
    let mut index = 0;
    while index < accepted
        invariant
            index <= accepted,
            accepted <= draft_tokens@.len(),
            emitted@ == draft_tokens@.subrange(0, index as int),
        decreases accepted - index,
    {
        emitted.push(draft_tokens[index]);
        index += 1;
    }
    emitted.push(correction_or_bonus);

    Ok(GreedyCommit {
        accepted_draft_tokens: accepted,
        emitted_tokens: emitted,
        target_correction_or_bonus: correction_or_bonus,
    })
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{verify_greedy_round, GreedyVerificationError};

    #[test]
    fn first_mismatch_emits_only_target_correction() {
        let commit = verify_greedy_round(&[3, 4, 5], &[9, 4, 5, 6]).unwrap();
        assert_eq!(commit.accepted_draft_tokens(), 0);
        assert_eq!(commit.emitted_tokens(), &[9]);
    }

    #[test]
    fn partial_match_commits_prefix_then_correction() {
        let commit = verify_greedy_round(&[3, 4, 5], &[3, 4, 9, 6]).unwrap();
        assert_eq!(commit.accepted_draft_tokens(), 2);
        assert_eq!(commit.emitted_tokens(), &[3, 4, 9]);
    }

    #[test]
    fn complete_match_emits_target_bonus() {
        let commit = verify_greedy_round(&[3, 4, 5], &[3, 4, 5, 6]).unwrap();
        assert_eq!(commit.accepted_draft_tokens(), 3);
        assert_eq!(commit.emitted_tokens(), &[3, 4, 5, 6]);
        assert_eq!(commit.target_correction_or_bonus(), 6);
    }

    #[test]
    fn incomplete_target_verification_is_rejected() {
        assert_eq!(
            verify_greedy_round(&[3, 4], &[3, 4]),
            Err(GreedyVerificationError::IncorrectTargetChoiceCount {
                expected: 3,
                actual: 2,
            })
        );
    }
}
