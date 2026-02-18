//! ============================================================================
//! Statistics models for functional analysis
//! ============================================================================

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Functional statistics response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionalStats {
    /// Job ID
    pub job_id: String,
    /// Total number of sequences
    pub total_sequences: usize,
    /// Number of annotated sequences
    pub annotated_sequences: usize,
    /// Top genes by frequency
    pub top_genes: Vec<CountItem>,
    /// Top products by frequency
    pub top_products: Vec<CountItem>,
    /// COG category distribution
    pub cog_categories: Vec<CogCategory>,
    /// EC number distribution (top-level classes)
    pub ec_classes: Vec<CountItem>,
    /// GO term distribution by ontology
    pub go_terms: GoTermStats,
}

/// A counted item (name + count)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CountItem {
    pub name: String,
    pub count: usize,
}

/// COG category with description
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CogCategory {
    pub code: String,
    pub name: String,
    pub count: usize,
}

/// GO terms grouped by ontology
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GoTermStats {
    /// Biological Process (GO:0008150)
    pub biological_process: Vec<CountItem>,
    /// Molecular Function (GO:0003674)
    pub molecular_function: Vec<CountItem>,
    /// Cellular Component (GO:0005575)
    pub cellular_component: Vec<CountItem>,
}

impl Default for GoTermStats {
    fn default() -> Self {
        Self {
            biological_process: Vec::new(),
            molecular_function: Vec::new(),
            cellular_component: Vec::new(),
        }
    }
}

/// COG category code to name mapping
pub fn cog_category_name(code: &str) -> &'static str {
    match code {
        "A" => "RNA processing and modification",
        "B" => "Chromatin structure and dynamics",
        "C" => "Energy production and conversion",
        "D" => "Cell cycle control, cell division",
        "E" => "Amino acid transport and metabolism",
        "F" => "Nucleotide transport and metabolism",
        "G" => "Carbohydrate transport and metabolism",
        "H" => "Coenzyme transport and metabolism",
        "I" => "Lipid transport and metabolism",
        "J" => "Translation, ribosomal structure",
        "K" => "Transcription",
        "L" => "Replication, recombination and repair",
        "M" => "Cell wall/membrane/envelope biogenesis",
        "N" => "Cell motility",
        "O" => "Post-translational modification, chaperones",
        "P" => "Inorganic ion transport and metabolism",
        "Q" => "Secondary metabolites biosynthesis",
        "R" => "General function prediction only",
        "S" => "Function unknown",
        "T" => "Signal transduction mechanisms",
        "U" => "Intracellular trafficking, secretion",
        "V" => "Defense mechanisms",
        "W" => "Extracellular structures",
        "X" => "Mobilome: prophages, transposons",
        "Y" => "Nuclear structure",
        "Z" => "Cytoskeleton",
        _ => "Unknown category",
    }
}

/// EC class (first digit) to name mapping
pub fn ec_class_name(class: &str) -> &'static str {
    match class {
        "1" => "Oxidoreductases",
        "2" => "Transferases",
        "3" => "Hydrolases",
        "4" => "Lyases",
        "5" => "Isomerases",
        "6" => "Ligases",
        "7" => "Translocases",
        _ => "Unknown class",
    }
}
