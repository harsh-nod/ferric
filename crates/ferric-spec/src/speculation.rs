use core::fmt;

pub type TokenId = u32;

/// The observable result of one greedy speculative verification round.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreedyCommit {
    accepted_draft_tokens: usize,
    emitted_tokens: Vec<TokenId>,
    target_correction_or_bonus: TokenId,
}

impl GreedyCommit {
    #[must_use]
    pub const fn accepted_draft_tokens(&self) -> usize {
        self.accepted_draft_tokens
    }

    #[must_use]
    pub fn emitted_tokens(&self) -> &[TokenId] {
        &self.emitted_tokens
    }

    #[must_use]
    pub const fn target_correction_or_bonus(&self) -> TokenId {
        self.target_correction_or_bonus
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GreedyVerificationError {
    MissingTargetBonus,
    IncorrectTargetChoiceCount { expected: usize, actual: usize },
}

impl fmt::Display for GreedyVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTargetBonus => {
                formatter.write_str("a speculative round requires at least one target choice")
            }
            Self::IncorrectTargetChoiceCount { expected, actual } => write!(
                formatter,
                "target choices must contain one choice per draft token plus one bonus: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for GreedyVerificationError {}

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
) -> Result<GreedyCommit, GreedyVerificationError> {
    if target_choices.is_empty() {
        return Err(GreedyVerificationError::MissingTargetBonus);
    }

    let expected = draft_tokens.len() + 1;
    if target_choices.len() != expected {
        return Err(GreedyVerificationError::IncorrectTargetChoiceCount {
            expected,
            actual: target_choices.len(),
        });
    }

    let accepted = draft_tokens
        .iter()
        .zip(target_choices.iter())
        .take_while(|(draft, target)| draft == target)
        .count();

    let correction_or_bonus = target_choices[accepted];
    let mut emitted = draft_tokens[..accepted].to_vec();
    emitted.push(correction_or_bonus);

    Ok(GreedyCommit {
        accepted_draft_tokens: accepted,
        emitted_tokens: emitted,
        target_correction_or_bonus: correction_or_bonus,
    })
}

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
