use crate::types::{TranscriptWord, WordRange};

/// Identifies filler words ("um", "uh", "er", "ah", etc.) in a transcript
/// and returns the retained word ranges that exclude these filler words.
pub fn purge_filler_words(
    words: &[TranscriptWord],
    fillers: &[&str],
) -> Vec<WordRange> {
    if words.is_empty() {
        return Vec::new();
    }

    let default_fillers = ["um", "uh", "er", "ah", "umm", "uhh"];
    let filler_list = if fillers.is_empty() {
        &default_fillers[..]
    } else {
        fillers
    };

    let mut keep_ranges = Vec::new();
    let mut range_start: Option<usize> = None;

    for (i, word) in words.iter().enumerate() {
        // Normalize word text (lowercase, stripped of punctuation)
        let clean: String = word
            .word
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase();

        let is_filler = filler_list.iter().any(|&f| f.eq_ignore_ascii_case(&clean));

        if is_filler {
            if let Some(start) = range_start {
                keep_ranges.push(WordRange {
                    start_word_idx: start,
                    end_word_idx: i - 1,
                });
                range_start = None;
            }
        } else if range_start.is_none() {
            range_start = Some(i);
        }
    }

    if let Some(start) = range_start {
        keep_ranges.push(WordRange {
            start_word_idx: start,
            end_word_idx: words.len() - 1,
        });
    }

    keep_ranges
}
