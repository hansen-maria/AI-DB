//! ============================================================================
//! Statistics handlers for functional analysis
//! ============================================================================

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use std::collections::HashMap;

use crate::models::{
    cog_category_name, ec_class_name, CogCategory, CountItem, ErrorResponse, FunctionalStats,
    GoTermStats, JobStatus, SequenceInfo,
};
use crate::state::AppState;

/// Maximum number of items to return in each category
const MAX_ITEMS: usize = 20;

// ── Shared computation ────────────────────────────────────────────────────────

/// Compute functional statistics from a slice of sequences.
/// Extracted so both the JSON endpoint and the CSV export can share the logic.
pub fn compute_stats(job_id: &str, sequences: &[SequenceInfo]) -> FunctionalStats {
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

        // Count products (functions)
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

        // Count COG categories
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
                    *go_mf_counts.entry(go_term.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    let top_genes = sorted_count_items(gene_counts, MAX_ITEMS);
    let top_products = sorted_count_items(product_counts, MAX_ITEMS);

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

    // NOTE: GO term ontology namespace (MF / BP / CC) is not stored in the DB.
    // All terms are grouped under molecular_function until namespace data is
    // available at annotation time. The frontend resolves labels via QuickGO.
    let go_terms = GoTermStats {
        biological_process: Vec::new(),
        molecular_function: sorted_count_items(go_mf_counts, MAX_ITEMS),
        cellular_component: Vec::new(),
    };

    FunctionalStats {
        job_id: job_id.to_string(),
        total_sequences: sequences.len(),
        annotated_sequences: annotated_count,
        top_genes,
        top_products,
        cog_categories,
        ec_classes,
        go_terms,
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Get functional statistics for a job (JSON)
#[utoipa::path(
    get,
    path = "/api/job/{job_id}/stats",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)")
    ),
    responses(
        (status = 200, description = "Functional statistics", body = FunctionalStats),
        (status = 400, description = "Job not completed",     body = ErrorResponse),
        (status = 404, description = "Job not found",         body = ErrorResponse)
    )
)]
pub async fn get_job_stats(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let jobs = state.jobs();

    match jobs.get(&job_id) {
        Some(job) => {
            if job.status != JobStatus::Completed {
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

            let stats = compute_stats(&job_id, sequences);
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

/// Export functional statistics as a CSV file
#[utoipa::path(
    get,
    path = "/api/job/{job_id}/stats/export",
    tag = "Jobs",
    params(
        ("job_id" = String, Path, description = "Unique job ID (UUID)")
    ),
    responses(
        (status = 200, description = "CSV file download", content_type = "text/csv"),
        (status = 400, description = "Job not completed", body = ErrorResponse),
        (status = 403, description = "Not authorized",    body = ErrorResponse),
        (status = 404, description = "Job not found",     body = ErrorResponse)
    )
)]
pub async fn export_job_stats(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let jobs = state.jobs();

    match jobs.get(&job_id) {
        Some(job) => {
            if job.status != JobStatus::Completed {
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
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse::new("No sequence data available")),
                    )
                        .into_response()
                }
            };

            let stats = compute_stats(&job_id, sequences);
            let csv = generate_stats_csv(&stats, job.filename.as_deref());
            let base = job
                .filename
                .as_deref()
                .map(|f| {
                    f.trim_end_matches(".gz")
                        .trim_end_matches(".fasta")
                        .trim_end_matches(".fa")
                })
                .unwrap_or("results");
            let filename = format!("{}_stats.csv", base);

            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        &format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                csv,
            )
                .into_response()
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

// ── CSV generation ────────────────────────────────────────────────────────────

fn generate_stats_csv(stats: &FunctionalStats, filename: Option<&str>) -> String {
    let mut out = String::with_capacity(8 * 1024);
    let now = Utc::now().format("%Y-%m-%d %H:%M UTC");

    // Header block
    out.push_str("# AI-DB Functional Statistics Export\n");
    out.push_str(&format!("# Job ID: {}\n", stats.job_id));
    if let Some(f) = filename {
        out.push_str(&format!("# File: {}\n", f));
    }
    out.push_str(&format!("# Generated: {}\n\n", now));

    // Summary
    let rate = if stats.total_sequences > 0 {
        stats.annotated_sequences as f64 / stats.total_sequences as f64 * 100.0
    } else {
        0.0
    };
    out.push_str("## Summary\n");
    out.push_str("Field,Value\n");
    out.push_str(&format!("Total Sequences,{}\n", stats.total_sequences));
    out.push_str(&format!(
        "Annotated Sequences,{}\n",
        stats.annotated_sequences
    ));
    out.push_str(&format!("Annotation Rate (%),{:.1}\n\n", rate));

    // Top Genes
    out.push_str("## Top Genes\n");
    out.push_str("Rank,Gene,Count\n");
    for (i, item) in stats.top_genes.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{}\n",
            i + 1,
            csv_escape(&item.name),
            item.count
        ));
    }
    out.push('\n');

    // Top Products
    out.push_str("## Top Products / Functions\n");
    out.push_str("Rank,Product,Count\n");
    for (i, item) in stats.top_products.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{}\n",
            i + 1,
            csv_escape(&item.name),
            item.count
        ));
    }
    out.push('\n');

    // COG Categories
    out.push_str("## COG Functional Categories\n");
    out.push_str("Rank,Code,Category,Count\n");
    for (i, cat) in stats.cog_categories.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{},{}\n",
            i + 1,
            cat.code,
            csv_escape(&cat.name),
            cat.count
        ));
    }
    out.push('\n');

    // EC Classes
    out.push_str("## Enzyme Classes (EC)\n");
    out.push_str("Rank,EC Class,Count\n");
    for (i, item) in stats.ec_classes.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{}\n",
            i + 1,
            csv_escape(&item.name),
            item.count
        ));
    }
    out.push('\n');

    // GO Terms (all ontologies combined – namespace not stored server-side)
    let all_go: Vec<_> = stats
        .go_terms
        .molecular_function
        .iter()
        .chain(stats.go_terms.biological_process.iter())
        .chain(stats.go_terms.cellular_component.iter())
        .collect();

    if !all_go.is_empty() {
        out.push_str("## GO Terms\n");
        out.push_str("Rank,GO ID,Count\n");
        for (i, item) in all_go.iter().enumerate() {
            out.push_str(&format!("{},{},{}\n", i + 1, item.name, item.count));
        }
        out.push('\n');
    }

    out
}

/// Wrap a CSV field in quotes if it contains commas, quotes, or newlines.
#[inline]
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

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
