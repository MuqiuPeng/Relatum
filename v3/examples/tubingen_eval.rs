//! Run v3's iid causal-direction estimator on the Tübingen
//! Cause-Effect Pairs benchmark.
//!
//! Usage (from `v3/`):
//!     cargo run --release --example tubingen_eval -- ../tubingen-data
//!
//! Defaults to `../tubingen-data` if no path is given.

use relatum_v3::causal::{
    CausalDirection, evaluate, judge_pair, load_tubingen_dataset,
};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let data_dir = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("../tubingen-data");
    let pairmeta_path = Path::new(data_dir).join("pairmeta.txt");
    let data_dir_path = Path::new(data_dir);

    println!("Loading from: {}", data_dir);
    let pairs = match load_tubingen_dataset(&pairmeta_path, data_dir_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to load dataset: {e}");
            std::process::exit(1);
        }
    };
    println!("Loaded {} bivariate pairs (multivariate skipped).", pairs.len());

    let summary = evaluate(&pairs);
    println!(
        "Overall: {}/{} correct = {:.1}%",
        summary.correct,
        summary.n,
        summary.accuracy * 100.0
    );

    // Per-pair breakdown
    println!("\nPer-pair judgments:");
    println!("{:>10}  {:>10}  {:>10}  {}", "pair", "ground", "v3", "result");
    let (mut correct_sct, mut total_sct) = (0usize, 0usize);
    let (mut correct_tcs, mut total_tcs) = (0usize, 0usize);
    for pair in &pairs {
        let judged = judge_pair(pair);
        let ok = judged == pair.ground_truth;
        let mark = if ok { "ok" } else { "MISS" };
        let gt_str = match pair.ground_truth {
            CausalDirection::SourceCausesTarget => {
                total_sct += 1;
                if ok {
                    correct_sct += 1;
                }
                "S->T"
            }
            CausalDirection::TargetCausesSource => {
                total_tcs += 1;
                if ok {
                    correct_tcs += 1;
                }
                "T->S"
            }
        };
        let v3_str = match judged {
            CausalDirection::SourceCausesTarget => "S->T",
            CausalDirection::TargetCausesSource => "T->S",
        };
        println!(
            "{:>10}  {:>10}  {:>10}  {}",
            pair.source_label, gt_str, v3_str, mark
        );
    }

    println!("\nBreakdown by ground-truth direction:");
    println!(
        "  Source->Target: {}/{} = {:.1}%",
        correct_sct,
        total_sct,
        if total_sct > 0 {
            100.0 * correct_sct as f64 / total_sct as f64
        } else {
            0.0
        }
    );
    println!(
        "  Target->Source: {}/{} = {:.1}%",
        correct_tcs,
        total_tcs,
        if total_tcs > 0 {
            100.0 * correct_tcs as f64 / total_tcs as f64
        } else {
            0.0
        }
    );

    // Detection of degenerate "always predict X" bias
    let n_sct_judged = pairs
        .iter()
        .filter(|p| judge_pair(p) == CausalDirection::SourceCausesTarget)
        .count();
    println!(
        "\nv3 judgment distribution: S->T = {}, T->S = {}",
        n_sct_judged,
        pairs.len() - n_sct_judged
    );
}
