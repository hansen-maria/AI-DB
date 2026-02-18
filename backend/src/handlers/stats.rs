//! ============================================================================
//! Statistics handlers for functional analysis
//! ============================================================================

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::collections::HashMap;

use crate::models::{
    cog_category_name, ec_class_name, CogCategory, CountItem, ErrorResponse, FunctionalStats,
    GoTermStats,
};
use crate::state::AppState;

/// Maximum number of items to return in each category
const MAX_ITEMS: usize = 20;

/// Get functional statistics for a job
#[utoipa::path(
    get,
    path = "/api/job/{job_id}/stats",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)")
    ),
    responses(
        (status = 200, description = "Functional statistics", body = FunctionalStats),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 400, description = "Job not completed", body = ErrorResponse)
    )
)]
pub async fn get_job_stats(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let jobs = state.jobs();

    match jobs.get(&job_id) {
        Some(job) => {
            // Check if job is completed
            if job.status != crate::models::JobStatus::Completed {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new("Job is not yet completed")),
                )
                    .into_response();
            }

            let sequences = match &job.sequences {
                Some(seqs) => seqs,
                None => {
                    return (
                        StatusCode::OK,
                        Json(FunctionalStats {
                            job_id: job_id.clone(),
                            total_sequences: 0,
                            annotated_sequences: 0,
                            top_genes: Vec::new(),
                            top_products: Vec::new(),
                            cog_categories: Vec::new(),
                            ec_classes: Vec::new(),
                            go_terms: GoTermStats::default(),
                        }),
                    )
                        .into_response();
                }
            };

            // Count statistics
            let mut gene_counts: HashMap<String, usize> = HashMap::new();
            let mut product_counts: HashMap<String, usize> = HashMap::new();
            let mut cog_counts: HashMap<String, usize> = HashMap::new();
            let mut ec_class_counts: HashMap<String, usize> = HashMap::new();
            let mut go_mf_counts: HashMap<String, usize> = HashMap::new();

            let mut annotated_count = 0;

            for seq in sequences {
                let has_annotation = seq.gene.is_some()
                    || seq.product.is_some()
                    || seq.cog_category.is_some()
                    || seq.ec_ids.is_some()
                    || seq.go_ids.is_some();

                if has_annotation {
                    annotated_count += 1;
                }

                // Count genes
                if let Some(ref gene) = seq.gene {
                    if !gene.is_empty() {
                        *gene_counts.entry(gene.clone()).or_insert(0) += 1;
                    }
                }

                // Count products (truncate long descriptions)
                if let Some(ref product) = seq.product {
                    if !product.is_empty() {
                        let truncated = if product.len() > 50 {
                            format!("{}...", &product[..47])
                        } else {
                            product.clone()
                        };
                        *product_counts.entry(truncated).or_insert(0) += 1;
                    }
                }

                // Count COG categories (can be multiple letters)
                if let Some(ref cog) = seq.cog_category {
                    for c in cog.chars() {
                        if c.is_alphabetic() {
                            *cog_counts.entry(c.to_string()).or_insert(0) += 1;
                        }
                    }
                }

                // Count EC classes (first digit)
                if let Some(ref ec_ids) = seq.ec_ids {
                    for ec in ec_ids.split(',') {
                        let ec = ec.trim();
                        if let Some(first_digit) = ec.chars().next() {
                            if first_digit.is_ascii_digit() {
                                *ec_class_counts.entry(first_digit.to_string()).or_insert(0) += 1;
                            }
                        }
                    }
                }

                // Count GO terms by ontology
                if let Some(ref go_ids) = seq.go_ids {
                    for go_term in go_ids.split(',') {
                        let go_term = go_term.trim();
                        if go_term.starts_with("GO:") {
                            // Categorize by GO term prefix (simplified)
                            // In practice, you'd need a GO ontology lookup
                            // For now, we'll just count all terms
                            // BP: GO:0008150 subtree, MF: GO:0003674 subtree, CC: GO:0005575 subtree
                            // Simplified: count in molecular function by default
                            *go_mf_counts.entry(go_term.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }

            // Convert to sorted vectors
            let top_genes = sorted_count_items(gene_counts, MAX_ITEMS);
            let top_products = sorted_count_items(product_counts, MAX_ITEMS);

            // COG categories with descriptions
            let cog_categories: Vec<CogCategory> = {
                let mut items: Vec<_> = cog_counts
                    .into_iter()
                    .map(|(code, count)| CogCategory {
                        name: cog_category_name(&code).to_string(),
                        code,
                        count,
                    })
                    .collect();
                items.sort_by(|a, b| b.count.cmp(&a.count));
                items.truncate(MAX_ITEMS);
                items
            };

            // EC classes with descriptions
            let ec_classes: Vec<CountItem> = {
                let mut items: Vec<_> = ec_class_counts
                    .into_iter()
                    .map(|(class, count)| CountItem {
                        name: format!("EC {} - {}", class, ec_class_name(&class)),
                        count,
                    })
                    .collect();
                items.sort_by(|a, b| b.count.cmp(&a.count));
                items.truncate(MAX_ITEMS);
                items
            };

            // GO terms (simplified - all as molecular function for now)
            let go_terms = GoTermStats {
                biological_process: Vec::new(),
                molecular_function: sorted_count_items(go_mf_counts, MAX_ITEMS),
                cellular_component: Vec::new(),
            };

            let stats = FunctionalStats {
                job_id: job_id.clone(),
                total_sequences: sequences.len(),
                annotated_sequences: annotated_count,
                top_genes,
                top_products,
                cog_categories,
                ec_classes,
                go_terms,
            };

            (StatusCode::OK, Json(stats)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!(
                "Job with ID '{}' not found",
                job_id
            ))),
        )
            .into_response(),
    }
}

/// Convert HashMap to sorted Vec<CountItem>
fn sorted_count_items(counts: HashMap<String, usize>, max_items: usize) -> Vec<CountItem> {
    let mut items: Vec<_> = counts
        .into_iter()
        .map(|(name, count)| CountItem { name, count })
        .collect();
    items.sort_by(|a, b| b.count.cmp(&a.count));
    items.truncate(max_items);
    items
}
