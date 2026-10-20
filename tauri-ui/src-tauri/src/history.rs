//! Bounded output history for a terminal, so a window that attaches later (a detached terminal window) can show what was already printed.

/// Most recent output of one terminal, capped, plus a running byte count so a reader can tell which later events it has already seen.
#[derive(Debug, Default)]
pub struct TerminalHistory {
    buffer: Vec<u8>,
    /// Bytes ever appended, not just the ones still held. Events carry this value so a late listener can drop duplicates.
    total: u64,
}

/// About 512 KiB per terminal. Older output is dropped; a replay therefore starts at a line boundary, not mid-line.
pub const HISTORY_LIMIT: usize = 512 * 1024;

impl TerminalHistory {
    /// Appends output and returns the running total after it. Trims the oldest bytes once over `limit`, cutting at the next line break
    /// (or, if the oldest kept bytes contain none, at a UTF-8 character boundary) so a replay does not begin inside a line or character.
    pub fn push(&mut self, data: &[u8], limit: usize) -> u64 {
        self.total += data.len() as u64;
        self.buffer.extend_from_slice(data);
        if self.buffer.len() > limit {
            let excess = self.buffer.len() - limit;
            let mut cut = self.buffer[excess..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|i| excess + i + 1)
                .unwrap_or(excess);
            while cut < self.buffer.len() && (self.buffer[cut] & 0b1100_0000) == 0b1000_0000 {
                cut += 1; // skip UTF-8 continuation bytes so the text starts on a whole character
            }
            self.buffer.drain(..cut.min(self.buffer.len()));
        }
        self.total
    }

    /// The held output as text and the running total it corresponds to.
    pub fn snapshot(&self) -> (String, u64) {
        (
            String::from_utf8_lossy(&self.buffer).into_owned(),
            self.total,
        )
    }

    /// Forgets the held output but keeps counting, so a restarted terminal's later output always has a higher total than anything a
    /// detached window has already seen (otherwise that window would drop the new output as duplicates).
    pub fn reset(&mut self) {
        self.buffer = Vec::new();
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_everything_under_the_limit_and_counts_bytes() {
        let mut h = TerminalHistory::default();
        assert_eq!(h.push(b"hello\r\n", 100), 7);
        assert_eq!(h.push(b"world", 100), 12);
        let (text, total) = h.snapshot();
        assert_eq!(text, "hello\r\nworld");
        assert_eq!(total, 12);
    }

    #[test]
    fn total_keeps_counting_after_old_output_is_dropped() {
        let mut h = TerminalHistory::default();
        for _ in 0..100 {
            h.push(b"0123456789\n", 50);
        }
        let (text, total) = h.snapshot();
        assert_eq!(total, 1100, "the running total is not capped");
        assert!(text.len() <= 50, "{} bytes held", text.len());
        assert!(
            text.starts_with("0123456789"),
            "replay must start at a line, got {text:?}"
        );
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn trimming_never_starts_inside_a_multibyte_character() {
        let mut h = TerminalHistory::default();
        let line = "日本語のテキスト"; // no line breaks: forces the character-boundary path
        for _ in 0..40 {
            h.push(line.as_bytes(), 30);
        }
        let (text, _) = h.snapshot();
        assert!(
            !text.contains('\u{FFFD}'),
            "a half character was kept: {text:?}"
        );
        assert!(text.len() <= 30 + 3);
        assert!(text.chars().all(|c| line.contains(c)));
    }

    #[test]
    fn reset_drops_the_text_but_the_total_keeps_rising() {
        let mut h = TerminalHistory::default();
        h.push(b"before restart\n", 100);
        h.reset();
        assert!(h.is_empty());
        assert_eq!(h.snapshot(), (String::new(), 15));
        assert_eq!(
            h.push(b"after\n", 100),
            21,
            "later output must outrank anything seen before the reset"
        );
    }

    #[test]
    fn an_empty_history_is_empty_and_has_total_zero() {
        let h = TerminalHistory::default();
        assert!(h.is_empty());
        assert_eq!(h.len(), 0);
        assert_eq!(h.snapshot(), (String::new(), 0));
    }

    #[test]
    fn a_single_chunk_larger_than_the_limit_is_cut_to_a_line_boundary() {
        let mut h = TerminalHistory::default();
        let big: Vec<u8> = (0..1000)
            .flat_map(|i| format!("line {i:04}\n").into_bytes())
            .collect();
        h.push(&big, 200);
        let (text, total) = h.snapshot();
        assert_eq!(total, big.len() as u64);
        assert!(text.len() <= 200 && text.starts_with("line "));
        assert!(
            text.ends_with("line 0999\n"),
            "the newest output must be kept: {:?}",
            &text[text.len().saturating_sub(20)..]
        );
    }
}
