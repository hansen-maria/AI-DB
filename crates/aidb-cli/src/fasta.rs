//! Local checks of a FASTA file before it is uploaded.
//!
//! Catches the common mistakes in seconds instead of after a 100 MB upload:
//! not a file, empty, over the server's size limit, not FASTA, or nucleotide
//! instead of protein sequences. AI-DB annotates proteins; the nucleotide
//! path of the server is not part of the published scope, so the CLI only
//! accepts protein FASTA.
//!
//! Only the beginning of the file is inspected (`SAMPLE_BYTES`, decompressed
//! for `.gz`), so the check stays fast for large files.

use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

use aidb_client::MAX_UPLOAD_BYTES;
use flate2::read::GzDecoder;

use crate::error::{CliError, Result};

/// Decompressed bytes inspected from the start of the file.
const SAMPLE_BYTES: usize = 256 * 1024;

/// Residues needed before the nucleotide heuristic is applied.
const MIN_RESIDUES: usize = 100;

/// Share of A/C/G/T/U/N from which a sample counts as nucleotides. Real
/// proteins stay far below (A+C+G+T+N together ≈ 30 %).
const NUCLEOTIDE_SHARE_PERCENT: usize = 90;

/// A file that passed the checks.
#[derive(Debug, Clone)]
pub struct CheckedFile {
    pub path: PathBuf,
    /// Size on disk (what is uploaded)
    pub size: u64,
}

/// Runs all checks on `path`.
pub fn check(path: &Path) -> Result<CheckedFile> {
    let name = path.display();
    let metadata = std::fs::metadata(path).map_err(|e| match e.kind() {
        ErrorKind::NotFound => CliError::Input(format!("{name}: file not found")),
        _ => CliError::io(format!("cannot read {name}"), e),
    })?;
    if !metadata.is_file() {
        return Err(CliError::Input(format!("{name}: not a file")));
    }
    let size = metadata.len();
    if size == 0 {
        return Err(CliError::Input(format!("{name}: the file is empty")));
    }
    if size > MAX_UPLOAD_BYTES {
        return Err(CliError::Input(format!(
            "{name}: {} MB is above the server limit of {} MB; compress it with gzip \
             or split it into several files",
            size / (1024 * 1024),
            MAX_UPLOAD_BYTES / (1024 * 1024)
        )));
    }

    let (sample, complete) = read_sample(path)?;
    analyze(&String::from_utf8_lossy(&sample), complete)
        .map_err(|reason| CliError::Input(format!("{name}: {reason}")))?;

    Ok(CheckedFile {
        path: path.to_path_buf(),
        size,
    })
}

/// Reads the first `SAMPLE_BYTES` (decompressed if gzipped). The flag tells
/// whether the sample is the whole content.
fn read_sample(path: &Path) -> Result<(Vec<u8>, bool)> {
    let name = path.display();
    let io_error = |e| CliError::io(format!("cannot read {name}"), e);

    let mut file = File::open(path).map_err(io_error)?;
    let mut magic = [0u8; 2];
    let gzip = file.read(&mut magic).map_err(io_error)? == 2 && magic == [0x1f, 0x8b];
    let file = File::open(path).map_err(io_error)?; // start over

    let reader: Box<dyn Read> = if gzip {
        Box::new(GzDecoder::new(file))
    } else {
        Box::new(file)
    };
    let mut sample = Vec::with_capacity(SAMPLE_BYTES);
    reader
        .take(SAMPLE_BYTES as u64 + 1)
        .read_to_end(&mut sample)
        .map_err(|e| {
            if gzip {
                CliError::Input(format!("{name}: not a valid gzip file ({e})"))
            } else {
                io_error(e)
            }
        })?;

    let complete = sample.len() <= SAMPLE_BYTES;
    sample.truncate(SAMPLE_BYTES);
    Ok((sample, complete))
}

/// Checks the (decompressed) start of a file. `complete` is true when `text`
/// is the whole file. Returns the reason on rejection.
fn analyze(text: &str, complete: bool) -> std::result::Result<(), String> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    match lines.next() {
        Some(first) if first.starts_with('>') => {}
        Some(_) => {
            return Err("does not look like FASTA (the first line must start with '>')".to_string())
        }
        None => return Err("contains no sequences".to_string()),
    }

    let (mut residues, mut nucleotides) = (0usize, 0usize);
    for line in lines.filter(|l| !l.starts_with('>')) {
        for c in line.chars().filter(char::is_ascii_alphabetic) {
            residues += 1;
            if matches!(c.to_ascii_uppercase(), 'A' | 'C' | 'G' | 'T' | 'U' | 'N') {
                nucleotides += 1;
            }
        }
    }

    if residues == 0 && complete {
        return Err("contains headers but no sequence data".to_string());
    }
    if residues >= MIN_RESIDUES && nucleotides * 100 >= residues * NUCLEOTIDE_SHARE_PERCENT {
        return Err(format!(
            "looks like nucleotide sequences ({} % A/C/G/T/U/N). AI-DB annotates protein \
             sequences: translate the genes first (e.g. with Bakta or Prodigal) and submit \
             the protein FASTA (.faa)",
            nucleotides * 100 / residues
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const PROTEIN: &str = ">WP_000001.1 hypothetical protein\n\
        MKRISTTITTTITITTGNGAGMKLLVSKQAHELGFDVIFVPSHQLAHEIRHTRHTLLANG\n\
        LKLEEIIGEDPIFLVDHHSEDVFGIANGFRFFAMHKAIKAMRSDKFVATIAGDG\n\
        >WP_000002.1 DNA-binding protein\n\
        MSKGQLEEAVLNYLKEHGEAVTVAEVSEQLGISRPTVSRVLRKLEKEGLIERTGRGWHL\n";

    fn dna(len: usize) -> String {
        format!(">contig_1\n{}\n", "ACGTTGCAAN".repeat(len / 10))
    }

    #[test]
    fn protein_fasta_is_accepted() {
        assert_eq!(analyze(PROTEIN, true), Ok(()));
        // Blank lines before the first header are fine
        assert_eq!(analyze(&format!("\n\n{PROTEIN}"), true), Ok(()));
    }

    #[test]
    fn nucleotide_fasta_is_rejected() {
        let err = analyze(&dna(500), true).unwrap_err();
        assert!(err.contains("nucleotide"), "{err}");
        // Lowercase and RNA too
        assert!(analyze(&dna(500).to_lowercase().replace('t', "u"), true).is_err());
    }

    #[test]
    fn short_nucleotide_like_samples_are_not_judged() {
        // Too few residues for a reliable call
        assert_eq!(analyze(">x\nACGTACGT\n", true), Ok(()));
    }

    #[test]
    fn non_fasta_and_empty_input_are_rejected() {
        assert!(analyze("id\tsequence\nx\tMKRI\n", true)
            .unwrap_err()
            .contains("FASTA"));
        assert!(analyze("", true).is_err());
        assert!(analyze(">only_header\n>another\n", true).is_err());
        // An incomplete sample with only headers so far is not rejected
        assert_eq!(analyze(">only_header\n", false), Ok(()));
    }

    #[test]
    fn files_are_checked_including_gzip() {
        let dir = tempfile::tempdir().unwrap();

        let plain = dir.path().join("proteins.faa");
        std::fs::write(&plain, PROTEIN).unwrap();
        assert_eq!(check(&plain).unwrap().size, PROTEIN.len() as u64);

        let gz = dir.path().join("genome.fna.gz");
        let mut encoder = flate2::write::GzEncoder::new(
            File::create(&gz).unwrap(),
            flate2::Compression::default(),
        );
        encoder.write_all(dna(1000).as_bytes()).unwrap();
        encoder.finish().unwrap();
        assert!(matches!(check(&gz), Err(CliError::Input(m)) if m.contains("nucleotide")));

        let broken = dir.path().join("broken.faa.gz");
        std::fs::write(&broken, [0x1f, 0x8b, 0x00, 0x01, 0x02]).unwrap();
        assert!(matches!(check(&broken), Err(CliError::Input(m)) if m.contains("gzip")));
    }

    #[test]
    fn missing_empty_and_directory_inputs_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            check(&dir.path().join("nope.faa")),
            Err(CliError::Input(_))
        ));
        assert!(matches!(check(dir.path()), Err(CliError::Input(_))));
        let empty = dir.path().join("empty.faa");
        std::fs::write(&empty, "").unwrap();
        assert!(matches!(check(&empty), Err(CliError::Input(_))));
    }
}
