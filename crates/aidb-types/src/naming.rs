//! ============================================================================
//! File names of downloaded results
//!
//! Shared by the web download (`Content-Disposition`) and the CLI, so a result
//! gets the same name either way: `<base>[_filtered]_annotations.<ext>`.
//! ============================================================================

/// Safe base name for result files, derived from a job or input file name.
///
/// * directories are dropped (`data/run1/x.faa` → `x`)
/// * `.gz` and one FASTA extension are removed (`.fasta`, `.faa`, `.fas`,
///   `.fa`, `.fna`, `.ffn`, `.txt`, any case)
/// * every character other than ASCII letters, digits, `-`, `_` and `.` becomes
///   `_`, so the name is portable and valid in an HTTP header
/// * an empty result becomes `results`
pub fn result_base_name(name: &str) -> String {
    let mut base = name.trim();
    base = base.rsplit(['/', '\\']).next().unwrap_or(base);
    if let Some(stripped) = strip_suffix_ignore_case(base, ".gz") {
        base = stripped;
    }
    for ext in [".fasta", ".faa", ".fas", ".fa", ".fna", ".ffn", ".txt"] {
        if let Some(stripped) = strip_suffix_ignore_case(base, ext) {
            base = stripped;
            break;
        }
    }
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('.');
    if cleaned.is_empty() {
        "results".to_string()
    } else {
        cleaned.to_string()
    }
}

/// `<base>[_filtered]_annotations.<extension>`
pub fn result_file_name(base: &str, filtered: bool, extension: &str) -> String {
    let suffix = if filtered { "_filtered" } else { "" };
    format!("{base}{suffix}_annotations.{extension}")
}

fn strip_suffix_ignore_case<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
    let cut = s.len().checked_sub(suffix.len())?;
    (s.is_char_boundary(cut) && s[cut..].eq_ignore_ascii_case(suffix)).then(|| &s[..cut])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_names_drop_paths_and_fasta_extensions() {
        assert_eq!(result_base_name("proteins.faa"), "proteins");
        assert_eq!(result_base_name("sample_01.faa.gz"), "sample_01");
        assert_eq!(result_base_name("Genome.FASTA.GZ"), "Genome");
        assert_eq!(result_base_name("genes.fna"), "genes");
        assert_eq!(result_base_name("direct_input"), "direct_input");
    }

    #[test]
    fn base_names_are_safe_for_files_and_headers() {
        assert_eq!(
            result_base_name("data/run 3/E. coli K-12.fa"),
            "E._coli_K-12"
        );
        assert_eq!(result_base_name("../../etc/passwd"), "passwd");
        assert_eq!(result_base_name(".."), "results");
        assert_eq!(result_base_name(""), "results");
        assert_eq!(result_base_name("Stämme.faa"), "St_mme");
        assert_eq!(result_base_name("x\";evil=1.faa"), "x__evil_1");
    }

    #[test]
    fn file_names_mark_filtered_results() {
        assert_eq!(
            result_file_name("proteins", false, "tsv"),
            "proteins_annotations.tsv"
        );
        assert_eq!(
            result_file_name("proteins", true, "fasta"),
            "proteins_filtered_annotations.fasta"
        );
    }
}
