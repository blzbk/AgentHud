//! Estimate of the Claude 5-hour window, the way `ccusage` computes its
//! blocks: Anthropic does not publish when the window starts.
//!
//! A block starts at the hour of its first message, floored, and lasts 5 h.
//! A message more than 5 h after the start of the block, or after the
//! previous message, opens a new block.

use std::collections::HashSet;

use domain::EpochSeconds;

use crate::transcript::UsageEntry;

/// Length of a block.
pub const BLOCK_SECONDS: EpochSeconds = 5 * 3600;

const HOUR_SECONDS: EpochSeconds = 3600;

/// The latest block of messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    pub start: EpochSeconds,
    /// `start` + 5 h: the estimated reset. A block whose end has passed is
    /// expired.
    pub end: EpochSeconds,
    pub tokens: u64,
    /// Time of the last message of the block.
    pub last_message: EpochSeconds,
}

/// Latest block of `entries`, in any order. Entries sharing a key count
/// once. `None` without any entry.
#[must_use]
pub fn latest_block<'a>(entries: impl IntoIterator<Item = &'a UsageEntry>) -> Option<Block> {
    let mut seen = HashSet::new();
    let mut unique: Vec<&UsageEntry> = entries
        .into_iter()
        .filter(|entry| entry.key.as_ref().is_none_or(|key| seen.insert(key)))
        .collect();
    unique.sort_by_key(|entry| entry.timestamp);

    let mut block: Option<Block> = None;
    for entry in unique {
        let current = block.filter(|block| {
            entry.timestamp - block.start <= BLOCK_SECONDS
                && entry.timestamp - block.last_message <= BLOCK_SECONDS
        });
        let mut next = current.unwrap_or_else(|| {
            let start = entry.timestamp - entry.timestamp.rem_euclid(HOUR_SECONDS);
            Block {
                start,
                end: start + BLOCK_SECONDS,
                tokens: 0,
                last_message: entry.timestamp,
            }
        });
        next.tokens = next.tokens.saturating_add(entry.tokens);
        next.last_message = entry.timestamp;
        block = Some(next);
    }
    block
}

#[cfg(test)]
mod tests {
    use super::{latest_block, Block, BLOCK_SECONDS};
    use crate::transcript::UsageEntry;

    /// `2026-11-03T12:00:00Z`.
    const NOON: i64 = 1_793_707_200;

    fn entry(key: Option<&str>, timestamp: i64, tokens: u64) -> UsageEntry {
        UsageEntry {
            key: key.map(str::to_owned),
            timestamp,
            tokens,
        }
    }

    #[test]
    fn sums_the_tokens_from_the_floored_hour() {
        let entries = [
            entry(Some("a"), NOON + 2 * 3600, 30),
            entry(Some("b"), NOON + 600, 10),
            entry(None, NOON + 3600, 20),
        ];

        assert_eq!(
            latest_block(&entries),
            Some(Block {
                start: NOON,
                end: NOON + BLOCK_SECONDS,
                tokens: 60,
                last_message: NOON + 2 * 3600,
            })
        );
    }

    #[test]
    fn counts_a_repeated_message_once() {
        let entries = [
            entry(Some("a"), NOON + 60, 10),
            entry(Some("a"), NOON + 61, 10),
            entry(Some("a"), NOON + 62, 10),
            entry(Some("b"), NOON + 63, 5),
        ];

        assert_eq!(latest_block(&entries).map(|block| block.tokens), Some(15));
    }

    #[test]
    fn a_gap_of_more_than_five_hours_opens_a_block_at_the_floored_hour() {
        let later = NOON + BLOCK_SECONDS + 3600 + 1234;
        let entries = [entry(None, NOON + 60, 10), entry(None, later, 7)];

        assert_eq!(
            latest_block(&entries),
            Some(Block {
                start: NOON + BLOCK_SECONDS + 3600,
                end: NOON + 2 * BLOCK_SECONDS + 3600,
                tokens: 7,
                last_message: later,
            })
        );
    }

    #[test]
    fn continuous_activity_opens_a_block_after_five_hours() {
        let entries: Vec<UsageEntry> = (0..12)
            .map(|half_hours| entry(None, NOON + 600 + half_hours * 1800, 1))
            .collect();

        let block = latest_block(&entries).expect("a block");

        // Messages at 12:10 .. 17:40: the one at 17:10 is past 17:00.
        assert_eq!(block.start, NOON + BLOCK_SECONDS);
        assert_eq!(block.tokens, 2);
    }

    #[test]
    fn a_message_exactly_five_hours_later_stays_in_the_block() {
        let entries = [entry(None, NOON, 1), entry(None, NOON + BLOCK_SECONDS, 1)];

        assert_eq!(latest_block(&entries).map(|block| block.start), Some(NOON));
    }

    #[test]
    fn no_entry_has_no_block() {
        assert_eq!(latest_block(&[]), None);
    }
}
