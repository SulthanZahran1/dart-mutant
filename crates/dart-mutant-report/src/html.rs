//! Self-contained HTML report generator with per-file mutation heatmap.
//!
//! Produces a single HTML file with all CSS inlined — no external dependencies,
//! no internet required. Openable directly in any browser.
//!
//! # Features
//!
//! - Per-file mutation score heatmap (green = high kill rate, red = low)
//! - Summary table with MSI, killed/survived/timeout/equivalent/not_covered/compile_error counts
//! - Per-mutant detail table: ID, file, mutator, status, original code, mutated code, location
//! - Color-coded status badges

use anyhow::Result;

use crate::{
    count_by_status, mutation_score, status_css_class, status_display, MutantResult, MutantStatus,
};

// ---------------------------------------------------------------------------
// HTML escaping
// ---------------------------------------------------------------------------

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Per-file aggregation
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct FileSummary {
    path: String,
    total: usize,
    killed: usize,
    survived: usize,
    timeout: usize,
    equivalent: usize,
    not_covered: usize,
    compile_error: usize,
    score: f64, // MSI for this file (killed / (killed + survived + timeout) * 100)
}

fn aggregate_by_file(results: &[MutantResult]) -> Vec<FileSummary> {
    // Group by file, preserving first-seen order
    let mut files: Vec<String> = Vec::new();
    let mut groups: Vec<Vec<&MutantResult>> = Vec::new();

    for r in results {
        if let Some(pos) = files.iter().position(|f| f == &r.mutant.file_path) {
            groups[pos].push(r);
        } else {
            files.push(r.mutant.file_path.clone());
            groups.push(vec![r]);
        }
    }

    files
        .iter()
        .zip(groups.iter())
        .map(|(path, mutants)| {
            let killed = mutants
                .iter()
                .filter(|r| r.status == MutantStatus::Killed)
                .count();
            let survived = mutants
                .iter()
                .filter(|r| r.status == MutantStatus::Survived)
                .count();
            let timeout = mutants
                .iter()
                .filter(|r| r.status == MutantStatus::Timeout)
                .count();
            let equivalent = mutants
                .iter()
                .filter(|r| r.status == MutantStatus::Equivalent)
                .count();
            let not_covered = mutants
                .iter()
                .filter(|r| r.status == MutantStatus::NotCovered)
                .count();
            let compile_error = mutants
                .iter()
                .filter(|r| r.status == MutantStatus::CompileError)
                .count();

            let relevant = killed + survived + timeout;
            let score = if relevant == 0 {
                0.0
            } else {
                killed as f64 / relevant as f64 * 100.0
            };

            FileSummary {
                path: path.clone(),
                total: mutants.len(),
                killed,
                survived,
                timeout,
                equivalent,
                not_covered,
                compile_error,
                score,
            }
        })
        .collect()
}

/// Determine the heatmap color for a mutation score.
///
/// - ≥80% → green
/// - 50-79% → yellow
/// - 20-49% → orange
/// - <20% → red
fn heatmap_color(score: f64) -> &'static str {
    if score >= 80.0 {
        "#4caf50" // green
    } else if score >= 50.0 {
        "#ffc107" // yellow
    } else if score >= 20.0 {
        "#ff9800" // orange
    } else {
        "#f44336" // red
    }
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

/// Generate a self-contained HTML report from mutation results.
///
/// The HTML includes all CSS inline — no external dependencies. It shows:
/// - Summary header with MSI and status counts
/// - Per-file mutation heatmap table
/// - Per-mutant detail table with status badges
///
/// # Example
///
/// ```
/// use dart_mutant_core::{Mutant, MutantResult, MutantStatus};
/// use dart_mutant_report::html;
///
/// let results = vec![MutantResult {
///     mutant: Mutant {
///         id: "0".to_string(), file_path: "lib/math.dart".to_string(),
///         line: 1, column: 1,
///         operator: "AOR".to_string(),
///         original: "a + b".to_string(), replacement: "a - b".to_string(),
///         description: "AOR: + to -".to_string(),
///     },
///     status: MutantStatus::Killed,
///     covering_tests: vec![], message: None,
/// }];
///
/// let html = html::generate(&results).unwrap();
/// assert!(html.contains("<!DOCTYPE html>"));
/// ```
pub fn generate(results: &[MutantResult]) -> Result<String> {
    let counts = count_by_status(results);
    let msi = mutation_score(results);
    let file_summaries = aggregate_by_file(results);

    let mut html = String::with_capacity(16384);

    // --- HTML head ---
    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str("<title>dart_mutant — Mutation Testing Report</title>\n");
    html.push_str("<style>\n");
    html.push_str(CSS);
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");

    // --- Header ---
    html.push_str("<header>\n");
    html.push_str("<h1>🧬 dart_mutant — Mutation Testing Report</h1>\n");
    html.push_str(&format!(
        "<p class=\"generated\">Generated: {}</p>\n",
        html_escape(&chrono_now())
    ));
    html.push_str("</header>\n");

    // --- Summary section ---
    html.push_str("<section id=\"summary\">\n");
    html.push_str("<h2>Summary</h2>\n");
    html.push_str("<div class=\"summary-grid\">\n");

    // MSI card
    let msi_color = heatmap_color(msi);
    html.push_str(&format!(
        "<div class=\"card msi-card\" style=\"border-left-color:{}\">\n\
         <span class=\"card-label\">Mutation Score (MSI)</span>\n\
         <span class=\"card-value\" style=\"color:{}\">{:.1}%</span>\n\
         </div>\n",
        msi_color, msi_color, msi
    ));

    // Count cards
    html.push_str(&format!(
        "<div class=\"card\" style=\"border-left-color:#4caf50\">\n\
         <span class=\"card-label\">Killed</span>\n\
         <span class=\"card-value killed\">{}</span>\n\
         </div>\n",
        counts.killed
    ));
    html.push_str(&format!(
        "<div class=\"card\" style=\"border-left-color:#f44336\">\n\
         <span class=\"card-label\">Survived</span>\n\
         <span class=\"card-value survived\">{}</span>\n\
         </div>\n",
        counts.survived
    ));
    html.push_str(&format!(
        "<div class=\"card\" style=\"border-left-color:#ff9800\">\n\
         <span class=\"card-label\">Timeout</span>\n\
         <span class=\"card-value timeout\">{}</span>\n\
         </div>\n",
        counts.timeout
    ));
    html.push_str(&format!(
        "<div class=\"card\" style=\"border-left-color:#9e9e9e\">\n\
         <span class=\"card-label\">Equivalent</span>\n\
         <span class=\"card-value equivalent\">{}</span>\n\
         </div>\n",
        counts.equivalent
    ));
    html.push_str(&format!(
        "<div class=\"card\" style=\"border-left-color:#2196f3\">\n\
         <span class=\"card-label\">Not Covered</span>\n\
         <span class=\"card-value not-covered\">{}</span>\n\
         </div>\n",
        counts.not_covered
    ));
    html.push_str(&format!(
        "<div class=\"card\" style=\"border-left-color:#9c27b0\">\n\
         <span class=\"card-label\">Compile Error</span>\n\
         <span class=\"card-value compile-error\">{}</span>\n\
         </div>\n",
        counts.compile_error
    ));
    html.push_str(&format!(
        "<div class=\"card\" style=\"border-left-color:#607d8b\">\n\
         <span class=\"card-label\">Total Mutants</span>\n\
         <span class=\"card-value\">{}</span>\n\
         </div>\n",
        counts.total
    ));

    html.push_str("</div>\n"); // summary-grid

    // Status legend
    html.push_str("<div class=\"legend\">\n");
    html.push_str("<span class=\"badge killed\">Killed</span>\n");
    html.push_str("<span class=\"badge survived\">Survived</span>\n");
    html.push_str("<span class=\"badge timeout\">Timeout</span>\n");
    html.push_str("<span class=\"badge equivalent\">Equivalent</span>\n");
    html.push_str("<span class=\"badge not-covered\">Not Covered</span>\n");
    html.push_str("<span class=\"badge compile-error\">Compile Error</span>\n");
    html.push_str("</div>\n");

    html.push_str("</section>\n");

    // --- Per-file heatmap ---
    html.push_str("<section id=\"files\">\n");
    html.push_str("<h2>Per-File Mutation Heatmap</h2>\n");
    html.push_str("<table class=\"heatmap\">\n");
    html.push_str(
        "<thead><tr><th>File</th><th>Total</th><th>Killed</th><th>Survived</th>\n\
         <th>Timeout</th><th>Eq.</th><th>Not Cov.</th><th>Err.</th><th>MSI</th><th>Score Bar</th></tr></thead>\n",
    );
    html.push_str("<tbody>\n");

    for f in &file_summaries {
        let color = heatmap_color(f.score);
        let bar_width = f.score.min(100.0) as u32;
        html.push_str(&format!(
            "<tr>\n\
             <td class=\"file-path\">{}</td>\n\
             <td>{}</td>\n\
             <td class=\"killed\">{}</td>\n\
             <td class=\"survived\">{}</td>\n\
             <td class=\"timeout\">{}</td>\n\
             <td class=\"equivalent\">{}</td>\n\
             <td class=\"not-covered\">{}</td>\n\
             <td class=\"compile-error\">{}</td>\n\
             <td class=\"score\" style=\"color:{}\">{:.1}%</td>\n\
             <td><div class=\"bar-container\"><div class=\"bar\" style=\"width:{}%;background:{}\"></div></div></td>\n\
             </tr>\n",
            html_escape(&f.path),
            f.total,
            f.killed,
            f.survived,
            f.timeout,
            f.equivalent,
            f.not_covered,
            f.compile_error,
            color,
            f.score,
            bar_width,
            color
        ));
    }

    html.push_str("</tbody>\n</table>\n");
    html.push_str("</section>\n");

    // --- Per-mutant detail ---
    html.push_str("<section id=\"mutants\">\n");
    html.push_str("<h2>Mutant Details</h2>\n");
    html.push_str("<table class=\"mutants\">\n");
    html.push_str(
        "<thead><tr><th>ID</th><th>File</th><th>Line</th><th>Mutator</th>\n\
         <th>Status</th><th>Original</th><th>Mutated</th><th>Killing Tests</th></tr></thead>\n",
    );
    html.push_str("<tbody>\n");

    for r in results {
        let status_class = status_css_class(r.status);
        let status_disp = status_display(r.status);
        let killing_tests = if r.covering_tests.is_empty() {
            "—".to_string()
        } else {
            r.covering_tests.join(", ")
        };

        html.push_str(&format!(
            "<tr class=\"mutant-row {}\">\n\
             <td class=\"mutant-id\">{}</td>\n\
             <td class=\"file-path\">{}</td>\n\
             <td>{}</td>\n\
             <td class=\"mutator\">{}</td>\n\
             <td><span class=\"badge {}\">{}</span></td>\n\
             <td><code class=\"code-block\">{}</code></td>\n\
             <td><code class=\"code-block\">{}</code></td>\n\
             <td class=\"killing-tests\">{}</td>\n\
             </tr>\n",
            status_class,
            html_escape(&r.mutant.id),
            html_escape(&r.mutant.file_path),
            r.mutant.line,
            html_escape(&r.mutant.operator),
            status_class,
            status_disp,
            html_escape(&r.mutant.original),
            html_escape(&r.mutant.replacement),
            html_escape(&killing_tests)
        ));
    }

    html.push_str("</tbody>\n</table>\n");
    html.push_str("</section>\n");

    // --- Footer ---
    html.push_str("<footer>\n");
    html.push_str("<p>Generated by <strong>dart_mutant</strong> — Mutation Testing for Dart</p>\n");
    html.push_str("</footer>\n");

    html.push_str("</body>\n</html>\n");

    Ok(html)
}

/// Generate a self-contained HTML report and write it to a file.
pub fn generate_to_file(results: &[MutantResult], path: &std::path::Path) -> Result<()> {
    let html = generate(results)?;
    crate::write_report_to_file(path, &html)
}

/// Get current timestamp as a human-readable string.
/// Uses a simple algorithm without external time crate deps.
fn chrono_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day, hour, minute, second) = epoch_to_datetime(secs as i64);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        year, month, day, hour, minute, second
    )
}

/// Convert Unix epoch seconds to (year, month, day, hour, minute, second).
/// Based on the civil-from-days algorithm by Howard Hinnant.
fn epoch_to_datetime(epoch: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = epoch.div_euclid(86400);
    let secs_of_day = epoch.rem_euclid(86400) as u32;

    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    let z = days + 719_468; // days from epoch to 0000-03-01
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = (z - era * 146_097) as u32; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]

    let year = if m <= 2 { y + 1 } else { y };

    (year, m, d, hour, minute, second)
}

// ---------------------------------------------------------------------------
// Inline CSS
// ---------------------------------------------------------------------------

const CSS: &str = r#"
:root {
  --bg: #1e1e2e;
  --fg: #cdd6f4;
  --card-bg: #313244;
  --border: #585b70;
  --link: #89b4fa;
}
* { margin: 0; padding: 0; box-sizing: border-box; }
body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: var(--bg); color: var(--fg);
  line-height: 1.6; padding: 1rem; max-width: 1400px; margin: 0 auto;
}
header { text-align: center; padding: 1.5rem 0; border-bottom: 1px solid var(--border); margin-bottom: 1.5rem; }
h1 { font-size: 1.8rem; }
h2 { font-size: 1.3rem; margin: 1.5rem 0 0.75rem; padding-bottom: 0.3rem; border-bottom: 1px solid var(--border); }
.generated { color: #a6adc8; font-size: 0.85rem; margin-top: 0.3rem; }
section { margin-bottom: 2rem; }
.summary-grid {
  display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 0.75rem; margin-bottom: 1rem;
}
.card {
  background: var(--card-bg); border-radius: 8px; padding: 1rem;
  border-left: 4px solid var(--border);
  display: flex; flex-direction: column; gap: 0.3rem;
}
.card-label { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #a6adc8; }
.card-value { font-size: 1.6rem; font-weight: bold; }
.msi-card { border-left-width: 6px; }
.legend { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 0.5rem; }
.badge {
  display: inline-block; padding: 0.15rem 0.5rem; border-radius: 4px;
  font-size: 0.75rem; font-weight: 600; text-transform: uppercase;
}
.badge.killed { background: #4caf50; color: #fff; }
.badge.survived { background: #f44336; color: #fff; }
.badge.timeout { background: #ff9800; color: #fff; }
.badge.equivalent { background: #9e9e9e; color: #fff; }
.badge.not-covered { background: #2196f3; color: #fff; }
.badge.compile-error { background: #9c27b0; color: #fff; }
.killed { color: #4caf50; }
.survived { color: #f44336; }
.timeout { color: #ff9800; }
.equivalent { color: #9e9e9e; }
.not-covered { color: #2196f3; }
.compile-error { color: #9c27b0; }
table {
  width: 100%; border-collapse: collapse; margin-top: 0.5rem;
  font-size: 0.85rem;
}
th, td {
  padding: 0.4rem 0.5rem; text-align: left;
  border-bottom: 1px solid var(--border);
}
th { font-weight: 600; color: #a6adc8; text-transform: uppercase; font-size: 0.72rem; letter-spacing: 0.03em; }
tbody tr:hover { background: rgba(255,255,255,0.05); }
.file-path { font-family: 'SF Mono', 'Fira Code', monospace; font-size: 0.8rem; }
.mutator { font-family: monospace; color: #fab387; }
.score { font-weight: bold; }
.bar-container { width: 100px; height: 12px; background: var(--card-bg); border-radius: 6px; overflow: hidden; }
.bar { height: 100%; border-radius: 6px; transition: width 0.3s; }
.mutant-id { font-family: monospace; color: #fab387; }
.code-block {
  font-family: 'SF Mono', 'Fira Code', monospace; font-size: 0.78rem;
  background: rgba(0,0,0,0.2); padding: 0.2rem 0.35rem; border-radius: 3px;
  display: inline-block; max-width: 200px; overflow-x: auto; white-space: nowrap;
}
.killing-tests { font-size: 0.78rem; color: #a6adc8; }
footer { text-align: center; padding: 1rem 0; border-top: 1px solid var(--border); color: #a6adc8; font-size: 0.8rem; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dart_mutant_core::{Mutant, MutantStatus};

    fn make_result(id: &str, status: MutantStatus) -> MutantResult {
        MutantResult {
            mutant: Mutant {
                id: id.to_string(),
                file_path: "lib/math.dart".to_string(),
                line: 1,
                column: 1,
                operator: "AOR".to_string(),
                original: "a + b".to_string(),
                replacement: "a - b".to_string(),
                description: "AOR: + → -".to_string(),
            },
            status,
            covering_tests: vec![],
            message: None,
        }
    }

    #[test]
    fn test_generates_html() {
        let results = vec![make_result("0", MutantStatus::Killed)];
        let html = generate(&results).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn test_contains_msi() {
        let results = vec![
            make_result("0", MutantStatus::Killed),
            make_result("1", MutantStatus::Survived),
        ];
        let html = generate(&results).unwrap();
        assert!(html.contains("Mutation Score (MSI)"));
        assert!(html.contains("50.0%"));
    }

    #[test]
    fn test_contains_heatmap() {
        let results = vec![
            make_result("0", MutantStatus::Killed),
            make_result("1", MutantStatus::Survived),
        ];
        let html = generate(&results).unwrap();
        assert!(html.contains("Per-File Mutation Heatmap"));
        assert!(html.contains("lib/math.dart"));
    }

    #[test]
    fn test_contains_mutant_details() {
        let results = vec![make_result("0", MutantStatus::Killed)];
        let html = generate(&results).unwrap();
        assert!(html.contains("Mutant Details"));
        assert!(html.contains("AOR"));
        assert!(html.contains("a + b"));
        assert!(html.contains("a - b"));
    }

    #[test]
    fn test_html_escaping() {
        let mut r = make_result("0", MutantStatus::Survived);
        r.mutant.original = "a < b && c > d".to_string();
        let html = generate(&[r]).unwrap();
        assert!(html.contains("&lt;"));
        assert!(html.contains("&gt;"));
        assert!(html.contains("&amp;"));
    }

    #[test]
    fn test_empty_results() {
        let html = generate(&[]).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Mutation Score (MSI)"));
        assert!(html.contains("0.0%"));
    }

    #[test]
    fn test_badge_classes() {
        let results = vec![
            make_result("0", MutantStatus::Killed),
            make_result("1", MutantStatus::Survived),
            make_result("2", MutantStatus::Timeout),
        ];
        let html = generate(&results).unwrap();
        assert!(html.contains("badge killed"));
        assert!(html.contains("badge survived"));
        assert!(html.contains("badge timeout"));
    }

    #[test]
    fn test_epoch_conversion() {
        let (y, m, d, h, mi, s) = epoch_to_datetime(1_735_689_600);
        assert_eq!(y, 2025);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
        assert_eq!(h, 0);
        assert_eq!(mi, 0);
        assert_eq!(s, 0);
    }
}
