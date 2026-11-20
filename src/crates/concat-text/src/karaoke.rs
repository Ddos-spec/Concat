// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ddos-spec and Concat contributors

//! Karaoke captions: the word being said, lit as it is said.
//!
//! A caption knows when it starts and ends; the words inside it are timed here
//! by spreading the caption's span over them in proportion to their length,
//! which is how a voice spends time closely enough to read as in sync, and
//! which needs nothing but the text. A transcriber that reports each word's
//! own times can hand those to [`TimedWord`] directly instead.
//!
//! The picture is not painted here. At a given time [`runs_at`] says which
//! characters wear the highlight, as the [`TitleRun`]s the title renderer
//! already paints, so a karaoke caption is an ordinary title whose runs change
//! with the playhead.

use crate::TitleRun;

/// One word with the time it is said and where it sits in the text.
#[derive(Clone, Debug, PartialEq)]
pub struct TimedWord {
    /// When the word starts, in seconds on the caption's own clock.
    pub start: f64,
    /// When it ends.
    pub end: f64,
    /// Its first character, counted in characters and not bytes, like [`TitleRun`].
    pub first: usize,
    /// One past its last character.
    pub last: usize,
}

/// How the highlight moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Only the word being said is lit.
    Current,
    /// Every word said so far stays lit, the sung-along fill.
    Filled,
}

/// Times each word of `text` inside the span `start..end`, in proportion to
/// its length. Punctuation that ends a clause (`, ; : . ! ?`) adds a short
/// pause after its word, taken from the same span.
pub fn spread(text: &str, start: f64, end: f64) -> Vec<TimedWord> {
    let span = (end - start).max(0.0);
    let mut words: Vec<(usize, usize, f64)> = Vec::new();
    let mut begin: Option<usize> = None;
    let mut count = 0usize;
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        count = i + 1;
        if c.is_whitespace() {
            if let Some(b) = begin.take() {
                words.push((b, i, weight(&chars[b..i])));
            }
        } else if begin.is_none() {
            begin = Some(i);
        }
    }
    if let Some(b) = begin {
        words.push((b, count, weight(&chars[b..count])));
    }
    let total: f64 = words.iter().map(|w| w.2).sum();
    if total <= 0.0 {
        return Vec::new();
    }
    let mut clock = start;
    words
        .into_iter()
        .map(|(first, last, w)| {
            let share = span * w / total;
            let word = TimedWord {
                start: clock,
                end: clock + share,
                first,
                last,
            };
            clock += share;
            word
        })
        .collect()
}

/// A word with its punctuation and case taken off, for comparing what was
/// printed with what was heard.
fn plain(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// The words of `text` timed from the words a transcriber heard, each given as
/// `(start, end, word)`. A heard word is matched to the next word of the text
/// that reads the same, ignoring case and punctuation; a heard word the text
/// does not have is skipped, and a word of the text that was not heard takes
/// the gap between its neighbours. The result is in the text's own order, so
/// it can be handed to [`runs_at`] as it is.
pub fn align(text: &str, heard: &[(f64, f64, &str)]) -> Vec<TimedWord> {
    let chars: Vec<char> = text.chars().collect();
    let mut words: Vec<(usize, usize, String)> = Vec::new();
    let mut begin: Option<usize> = None;
    for (i, c) in chars.iter().enumerate() {
        if c.is_whitespace() {
            if let Some(b) = begin.take() {
                words.push((b, i, plain(&chars[b..i].iter().collect::<String>())));
            }
        } else if begin.is_none() {
            begin = Some(i);
        }
    }
    if let Some(b) = begin {
        words.push((b, chars.len(), plain(&chars[b..].iter().collect::<String>())));
    }

    let mut times: Vec<Option<(f64, f64)>> = vec![None; words.len()];
    let mut next = 0usize;
    for (start, end, word) in heard {
        let wanted = plain(word);
        if wanted.is_empty() {
            continue;
        }
        if let Some(k) = (next..words.len()).find(|k| words[*k].2 == wanted) {
            times[k] = Some((*start, (*end).max(*start)));
            next = k + 1;
        }
    }

    // Words that were not heard sit in the gap between the ones around them.
    let mut out = Vec::with_capacity(words.len());
    let mut clock = times.iter().flatten().next().map_or(0.0, |t| t.0);
    for (k, (first, last, _)) in words.iter().enumerate() {
        let (start, end) = match times[k] {
            Some(t) => t,
            None => {
                let after = times[k + 1..].iter().flatten().next().map_or(clock, |t| t.0);
                (clock, after.max(clock))
            }
        };
        clock = end;
        out.push(TimedWord {
            start,
            end,
            first: *first,
            last: *last,
        });
    }
    out
}

/// A word's claim on the span: its letters, and a pause if a clause ends on it.
fn weight(word: &[char]) -> f64 {
    let letters = word.iter().filter(|c| c.is_alphanumeric()).count().max(1) as f64;
    let pause = match word.last() {
        Some(',' | ';' | ':') => 1.5,
        Some('.' | '!' | '?') => 3.0,
        _ => 0.0,
    };
    letters + pause
}

/// The runs that light the right words at `time`: `lit` is the highlight
/// colour (`#rrggbb[aa]`). Adjacent lit words are merged into one run.
pub fn runs_at(words: &[TimedWord], time: f64, mode: Mode, lit: &str) -> Vec<TitleRun> {
    let mut runs: Vec<TitleRun> = Vec::new();
    for word in words {
        let on = match mode {
            Mode::Current => time >= word.start && time < word.end,
            Mode::Filled => time >= word.start,
        };
        if !on {
            continue;
        }
        if let Some(last) = runs.last_mut() {
            // Only the gap between two lit words is a space; join across it.
            if last.end + 1 >= word.first {
                last.end = word.last;
                continue;
            }
        }
        runs.push(TitleRun {
            start: word.first,
            end: word.last,
            color: Some(lit.to_owned()),
            ..TitleRun::default()
        });
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_share_the_span_by_length_and_tile_it() {
        let words = spread("a bb cccc", 10.0, 17.0);
        assert_eq!(words.len(), 3);
        assert!((words[0].start - 10.0).abs() < 1e-9);
        assert!((words[2].end - 17.0).abs() < 1e-9);
        // Lengths 1:2:4 of 7 seconds.
        assert!((words[0].end - 11.0).abs() < 1e-9);
        assert!((words[1].end - 13.0).abs() < 1e-9);
        for pair in words.windows(2) {
            assert!((pair[0].end - pair[1].start).abs() < 1e-9);
        }
    }

    #[test]
    fn a_sentence_end_buys_a_pause() {
        let plain = spread("go now", 0.0, 6.0);
        let stopped = spread("go. now", 0.0, 6.0);
        assert!(stopped[0].end - stopped[0].start > plain[0].end - plain[0].start);
    }

    #[test]
    fn characters_are_counted_not_bytes() {
        let words = spread("héllo wörld", 0.0, 2.0);
        assert_eq!((words[0].first, words[0].last), (0, 5));
        assert_eq!((words[1].first, words[1].last), (6, 11));
    }

    #[test]
    fn current_mode_lights_only_the_word_being_said() {
        let words = spread("one two three", 0.0, 3.0);
        let runs = runs_at(&words, words[1].start + 0.01, Mode::Current, "#ffd60a");
        assert_eq!(runs.len(), 1);
        assert_eq!((runs[0].start, runs[0].end), (words[1].first, words[1].last));
        assert_eq!(runs[0].color.as_deref(), Some("#ffd60a"));
    }

    #[test]
    fn filled_mode_keeps_spoken_words_lit_as_one_run() {
        let words = spread("one two three", 0.0, 3.0);
        let runs = runs_at(&words, words[1].start + 0.01, Mode::Filled, "#ffd60a");
        assert_eq!(runs.len(), 1);
        assert_eq!((runs[0].start, runs[0].end), (0, words[1].last));
    }

    #[test]
    fn nothing_is_lit_before_the_caption_starts_and_empty_text_has_no_words() {
        let words = spread("one two", 5.0, 7.0);
        assert!(runs_at(&words, 4.0, Mode::Filled, "#fff").is_empty());
        assert!(spread("   ", 0.0, 1.0).is_empty());
    }

    #[test]
    fn heard_words_time_the_text_and_the_unheard_take_the_gap() {
        let heard = [(0.0, 0.4, "hello"), (0.5, 0.9, "world"), (1.6, 2.0, "test")];
        // "this is a" were not heard; the text keeps its punctuation and case.
        let words = align("Hello, World. This is a TEST", &heard);
        assert_eq!(words.len(), 6);
        assert_eq!((words[0].start, words[0].end), (0.0, 0.4));
        assert_eq!((words[1].start, words[1].end), (0.5, 0.9));
        assert!((words[2].start - 0.9).abs() < 1e-9 && (words[2].end - 1.6).abs() < 1e-9);
        assert!((words[4].start - 1.6).abs() < 1e-9);
        assert_eq!((words[5].start, words[5].end), (1.6, 2.0));
        assert_eq!((words[0].first, words[0].last), (0, 6));
        // Lit as it is said, in the text's order.
        let runs = runs_at(&words, 0.6, Mode::Filled, "#ffd60a");
        assert_eq!(runs.len(), 1);
        assert_eq!((runs[0].start, runs[0].end), (0, words[1].last));
    }

    #[test]
    fn a_heard_word_the_text_lacks_is_skipped() {
        let heard = [(0.0, 0.3, "uh"), (0.4, 0.8, "go"), (0.9, 1.2, "now")];
        let words = align("go now", &heard);
        assert_eq!(words.len(), 2);
        assert_eq!((words[0].start, words[1].end), (0.4, 1.2));
    }

    #[test]
    fn nothing_heard_nothing_timed_but_the_order_holds() {
        let words = align("one two", &[]);
        assert_eq!(words.len(), 2);
        assert!(words.iter().all(|w| w.start == 0.0 && w.end == 0.0));
    }
}
