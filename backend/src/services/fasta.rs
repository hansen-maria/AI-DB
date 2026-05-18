//! ============================================================================
//! FASTA parsing utilities
//! ============================================================================

use std::io::BufRead;

/// Batch size for processing sequences
pub const BATCH_SIZE: usize = 1000;

/// Maximum sequence length to prevent memory issues
pub const MAX_SEQUENCE_LENGTH: usize = 5_000_000; // 5 MB

/// Maximum results to store (prevent OOM)
pub const MAX_RESULTS: usize = 1_000_000;

/// Streaming FASTA parser that yields sequences one at a time
pub struct FastaIterator<R: BufRead> {
    reader: R,
    current_header: Option<String>,
    current_sequence: String,
    line_buffer: String,
    finished: bool,
}

impl<R: BufRead> FastaIterator<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            current_header: None,
            current_sequence: String::new(),
            line_buffer: String::new(),
            finished: false,
        }
    }
}

impl<R: BufRead> Iterator for FastaIterator<R> {
    type Item = (String, String);

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            self.line_buffer.clear();
            match self.reader.read_line(&mut self.line_buffer) {
                Ok(0) => {
                    // EOF reached
                    self.finished = true;
                    if let Some(header) = self.current_header.take() {
                        if !self.current_sequence.is_empty() {
                            let seq = std::mem::take(&mut self.current_sequence);
                            return Some((header, seq));
                        }
                    }
                    return None;
                }
                Ok(_) => {
                    let line = self.line_buffer.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if line.starts_with('>') {
                        // New sequence header
                        let new_header = line[1..]
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_string();

                        if let Some(header) = self.current_header.take() {
                            if !self.current_sequence.is_empty() {
                                let seq = std::mem::take(&mut self.current_sequence);
                                self.current_header = Some(new_header);
                                return Some((header, seq));
                            }
                        }
                        self.current_header = Some(new_header);
                        self.current_sequence.clear();
                    } else {
                        // Sequence line - append (with length limit)
                        if self.current_sequence.len() < MAX_SEQUENCE_LENGTH {
                            self.current_sequence.push_str(&line.to_uppercase());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading FASTA: {}", e);
                    self.finished = true;
                    return None;
                }
            }
        }
    }
}

/// Computes MD5 hash of a sequence and returns both hex string and bytes
pub fn compute_md5(sequence: &str) -> (String, Vec<u8>) {
    let digest = md5::compute(sequence.as_bytes());
    let hash_bytes = digest.0.to_vec();
    let hash_hex = format!("{:x}", digest);
    (hash_hex, hash_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_fasta_iterator_single_sequence() {
        let fasta = ">seq1\nACGT\nTGCA\n";
        let reader = Cursor::new(fasta);
        let mut iter = FastaIterator::new(reader);

        let (header, seq) = iter.next().unwrap();
        assert_eq!(header, "seq1");
        assert_eq!(seq, "ACGTTGCA");
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_fasta_iterator_multiple_sequences() {
        let fasta = ">seq1\nACGT\n>seq2\nTGCA\n";
        let reader = Cursor::new(fasta);
        let iter = FastaIterator::new(reader);

        let seqs: Vec<_> = iter.collect();
        assert_eq!(seqs.len(), 2);
        assert_eq!(seqs[0].0, "seq1");
        assert_eq!(seqs[0].1, "ACGT");
        assert_eq!(seqs[1].0, "seq2");
        assert_eq!(seqs[1].1, "TGCA");
    }

    #[test]
    fn test_compute_md5() {
        let (hex, bytes) = compute_md5("ACGT");
        assert_eq!(hex.len(), 32);
        assert_eq!(bytes.len(), 16);
    }
}
