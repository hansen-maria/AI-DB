//! ============================================================================
//! GFF3 export format following the GFF3 specification
//! ============================================================================

use crate::models::JobResponse;

/// Generate GFF3 output following the GFF3 specification
/// For protein annotations, each sequence is treated as a region with annotation features
pub fn generate_gff3(job: &JobResponse) -> String {
    let mut output = String::new();

    // GFF3 header (required)
    output.push_str("##gff-version 3\n");

    // Metadata as comments
    output.push_str("#!annotation-source AI-DB v1.0\n");
    output.push_str(&format!("#!job-id {}\n", job.job_id));
    if let Some(ref filename) = job.filename {
        output.push_str(&format!("#!original-file {}\n", filename));
    }
    output.push_str(&format!("#!date {}\n", job.created_at.format("%Y-%m-%d")));

    if let Some(ref sequences) = job.sequences {
        // First pass: declare all sequence regions
        for seq in sequences {
            let safe_seqid = sanitize_gff3_seqid(&seq.id);
            output.push_str(&format!(
                "##sequence-region {} 1 {}\n",
                safe_seqid, seq.length
            ));
        }

        // Separator between header and features
        output.push_str("###\n");

        // Second pass: output features
        for (idx, seq) in sequences.iter().enumerate() {
            let safe_seqid = sanitize_gff3_seqid(&seq.id);

            // Determine feature type based on annotation source (SOFA terms)
            // - polypeptide (SO:0000104): A sequence of amino acids
            // - protein_match (SO:0000349): A match to a protein sequence
            let (feature_type, score) = match seq.annotation_source.as_deref() {
                Some("hash_match") => ("protein_match", "."),
                Some("alignment") => ("protein_match", "."),
                _ => ("polypeptide", "."),
            };

            // Build attributes following GFF3 attribute conventions
            let mut attributes = Vec::new();

            // ID is required for features that may be referenced
            attributes.push(format!("ID=seq_{:06}", idx + 1));

            // Name attribute for display
            let display_name = sanitize_gff3_attribute(&seq.id);
            attributes.push(format!("Name={}", display_name));

            // Add gene name if present (standard GFF3 attribute)
            if let Some(ref gene) = seq.gene {
                let encoded = encode_gff3_attribute(gene);
                attributes.push(format!("gene={}", encoded));
            }

            // Add product/function description if present (standard GFF3 attribute)
            if let Some(ref product) = seq.product {
                let encoded = encode_gff3_attribute(product);
                attributes.push(format!("product={}", encoded));
            }

            // Add annotation note if present (legacy field)
            if let Some(ref annotation) = seq.annotation {
                let encoded = encode_gff3_attribute(annotation);
                attributes.push(format!("Note={}", encoded));
            }

            // Add database cross-references (Dbxref format: DB:ID)
            let mut dbxrefs = Vec::new();
            if let Some(ref uniparc) = seq.uniparc_id {
                dbxrefs.push(format!("UniParc:{}", uniparc));
            }
            if let Some(ref uniref) = seq.uniref100_id {
                dbxrefs.push(format!("UniRef100:{}", uniref));
            }
            if let Some(ref ncbi) = seq.ncbi_nrp_id {
                dbxrefs.push(format!("NCBI_NRP:{}", ncbi));
            }
            if !dbxrefs.is_empty() {
                attributes.push(format!("Dbxref={}", dbxrefs.join(",")));
            }

            // Add ontology term for annotation source
            if let Some(ref source) = seq.annotation_source {
                attributes.push(format!("source_type={}", source));
            }

            // GFF3 columns (tab-separated):
            // seqid, source, type, start, end, score, strand, phase, attributes
            // For proteins: strand is '.', phase is '.' (only relevant for CDS)
            output.push_str(&format!(
                "{}\tai-db\t{}\t1\t{}\t{}\t.\t.\t{}\n",
                safe_seqid,
                feature_type,
                seq.length,
                score,
                attributes.join(";")
            ));
        }
    }

    output
}

/// Sanitize sequence ID for use as GFF3 seqid (column 1)
/// seqid cannot contain whitespace, semicolons, equals signs, or percent signs (unencoded)
fn sanitize_gff3_seqid(id: &str) -> String {
    id.chars()
        .map(|c| match c {
            ' ' | '\t' | '\n' | '\r' => '_',
            ';' | '=' | '%' | '&' | ',' => '_',
            _ => c,
        })
        .collect()
}

/// Sanitize a value for use in GFF3 attributes (not URL-encoded, just cleaned)
fn sanitize_gff3_attribute(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            ';' | '=' | '&' | ',' | '\t' | '\n' | '\r' => '_',
            _ => c,
        })
        .collect()
}

/// URL-encode special characters in GFF3 attribute values
/// Required for: tab, newline, carriage return, semicolons, equals, percent, ampersand, comma
fn encode_gff3_attribute(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '%' => encoded.push_str("%25"),
            ';' => encoded.push_str("%3B"),
            '=' => encoded.push_str("%3D"),
            '&' => encoded.push_str("%26"),
            ',' => encoded.push_str("%2C"),
            '\t' => encoded.push_str("%09"),
            '\n' => encoded.push_str("%0A"),
            '\r' => encoded.push_str("%0D"),
            _ => encoded.push(c),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_gff3_seqid() {
        assert_eq!(sanitize_gff3_seqid("seq 1"), "seq_1");
        assert_eq!(sanitize_gff3_seqid("seq;1=2"), "seq_1_2");
    }

    #[test]
    fn test_encode_gff3_attribute() {
        assert_eq!(encode_gff3_attribute("a;b=c"), "a%3Bb%3Dc");
        assert_eq!(encode_gff3_attribute("50%"), "50%25");
    }
}
