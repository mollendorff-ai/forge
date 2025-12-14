//! Performance benchmarks for Forge
//!
//! Run with: cargo test --release perf_ -- --nocapture
//!
//! ## I/O Strategy
//!
//! Benchmarks use `/dev/shm` (RAM filesystem) instead of disk to isolate
//! CPU/memory performance from I/O latency. This is especially important
//! when developing on USB drives or HDDs where I/O would dominate timing.
//!
//! Real-world performance will vary based on storage:
//! - NVMe SSD: Similar to benchmark results
//! - SATA SSD: ~5-10% slower on parse phase
//! - USB drive: ~20-50% slower on parse phase
//! - Network storage: Highly variable
//!
//! The calculation phase is CPU-bound and unaffected by storage speed.

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::parser::parse_model;
use std::time::Instant;

/// Generate a large YAML model with N rows for benchmarking
fn generate_large_model(rows: usize) -> String {
    let mut yaml = String::with_capacity(rows * 100);
    yaml.push_str("_forge_version: \"1.0.0\"\n\n");

    // Table: transactions (the main large table)
    yaml.push_str("transactions:\n");

    // Column: unit_price
    yaml.push_str("  unit_price: [");
    for i in 0..rows {
        if i > 0 {
            yaml.push_str(", ");
        }
        yaml.push_str(&format!("{:.2}", 10.0 + (i % 100) as f64 * 0.5));
    }
    yaml.push_str("]\n");

    // Column: quantity
    yaml.push_str("  quantity: [");
    for i in 0..rows {
        if i > 0 {
            yaml.push_str(", ");
        }
        yaml.push_str(&format!("{}", 1 + (i % 50)));
    }
    yaml.push_str("]\n");

    // Column: unit_cost
    yaml.push_str("  unit_cost: [");
    for i in 0..rows {
        if i > 0 {
            yaml.push_str(", ");
        }
        yaml.push_str(&format!("{:.2}", 5.0 + (i % 80) as f64 * 0.3));
    }
    yaml.push_str("]\n");

    // Row-wise formulas (computed columns)
    yaml.push_str("  revenue: \"=unit_price * quantity\"\n");
    yaml.push_str("  cost: \"=unit_cost * quantity\"\n");
    yaml.push_str("  gross_profit: \"=revenue - cost\"\n");
    yaml.push_str("  profit: \"=gross_profit * 0.75\"\n");
    yaml.push_str("  margin: \"=profit / revenue\"\n");

    // Scalars section with aggregations
    yaml.push_str("\nsummary:\n");
    yaml.push_str("  tax_rate:\n");
    yaml.push_str("    value: 0.25\n");
    yaml.push_str("  total_revenue:\n");
    yaml.push_str("    value: null\n");
    yaml.push_str("    formula: \"=SUM(transactions.revenue)\"\n");
    yaml.push_str("  total_profit:\n");
    yaml.push_str("    value: null\n");
    yaml.push_str("    formula: \"=SUM(transactions.profit)\"\n");
    yaml.push_str("  avg_margin:\n");
    yaml.push_str("    value: null\n");
    yaml.push_str("    formula: \"=AVERAGE(transactions.margin)\"\n");

    yaml
}

/// Benchmark full calculation (parse + calculate) for given row count
fn bench_calculate(rows: usize) -> Result<(std::time::Duration, std::time::Duration), String> {
    let yaml = generate_large_model(rows);

    // Write to RAM disk (/dev/shm) for fastest I/O
    let temp_path = std::path::PathBuf::from(format!("/dev/shm/forge_bench_{}.yaml", rows));
    std::fs::write(&temp_path, yaml.as_bytes())
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    // Time parsing
    let parse_start = Instant::now();
    let model = parse_model(&temp_path).map_err(|e| format!("Model parse error: {}", e))?;
    let parse_time = parse_start.elapsed();

    // Time calculation
    let calc_start = Instant::now();
    let calculator = ArrayCalculator::new(model);
    let _result = calculator
        .calculate_all()
        .map_err(|e| format!("Calc error: {}", e))?;
    let calc_time = calc_start.elapsed();

    // Cleanup
    let _ = std::fs::remove_file(&temp_path);

    Ok((parse_time, calc_time))
}

#[test]
#[ignore] // Run with: cargo test perf_baseline_100_rows -- --ignored --nocapture
fn perf_baseline_100_rows() {
    let (parse, calc) = bench_calculate(100).expect("Calculation failed");
    let total = parse + calc;
    println!(
        "\n📊 100 rows: parse={:?}, calc={:?}, total={:?}",
        parse, calc, total
    );
    assert!(
        total.as_millis() < 2000,
        "100 rows should complete in <2s, got {:?}",
        total
    );
}

#[test]
#[ignore] // Run with: cargo test perf_baseline_1000_rows -- --ignored --nocapture
fn perf_baseline_1000_rows() {
    let (parse, calc) = bench_calculate(1000).expect("Calculation failed");
    let total = parse + calc;
    println!(
        "\n📊 1,000 rows: parse={:?}, calc={:?}, total={:?}",
        parse, calc, total
    );
    assert!(
        total.as_millis() < 10000,
        "1000 rows should complete in <10s, got {:?}",
        total
    );
}

#[test]
#[ignore] // Run with: cargo test perf_baseline_5000_rows -- --ignored --nocapture
fn perf_baseline_5000_rows() {
    let (parse, calc) = bench_calculate(5000).expect("Calculation failed");
    let total = parse + calc;
    println!(
        "\n📊 5,000 rows: parse={:?}, calc={:?}, total={:?}",
        parse, calc, total
    );
    assert!(
        total.as_millis() < 60000,
        "5000 rows should complete in <60s, got {:?}",
        total
    );
}

#[test]
#[ignore] // Run with --ignored for full benchmark
fn perf_baseline_10000_rows() {
    let (parse, calc) = bench_calculate(10000).expect("Calculation failed");
    let total = parse + calc;
    println!(
        "\n📊 10,000 rows: parse={:?}, calc={:?}, total={:?}",
        parse, calc, total
    );
    assert!(
        total.as_millis() < 1000,
        "10000 rows should complete in <1s, got {:?}",
        total
    );
}

#[test]
#[ignore]
fn perf_baseline_50000_rows() {
    let (parse, calc) = bench_calculate(50000).expect("Calculation failed");
    let total = parse + calc;
    println!(
        "\n📊 50,000 rows: parse={:?}, calc={:?}, total={:?}",
        parse, calc, total
    );
    assert!(
        total.as_millis() < 5000,
        "50000 rows should complete in <5s, got {:?}",
        total
    );
}

#[test]
#[ignore]
fn perf_baseline_100000_rows() {
    let (parse, calc) = bench_calculate(100000).expect("Calculation failed");
    let total = parse + calc;
    println!(
        "\n📊 100,000 rows: parse={:?}, calc={:?}, total={:?}",
        parse, calc, total
    );
    assert!(
        total.as_millis() < 10000,
        "100000 rows should complete in <10s, got {:?}",
        total
    );
}

#[test]
#[ignore] // Run with: cargo test perf_report -- --ignored --nocapture
fn perf_report() {
    println!("\n");
    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║              FORGE PERFORMANCE BASELINE REPORT                        ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║  Rows   │   Parse    │    Calc    │   Total    │   Rows/sec           ║");
    println!("╠═════════╪════════════╪════════════╪════════════╪══════════════════════╣");

    let sizes = [100, 500, 1000, 2000, 5000, 10000];
    for &size in &sizes {
        match bench_calculate(size) {
            Ok((parse, calc)) => {
                let total = parse + calc;
                let ms = total.as_millis();
                let rows_per_sec = if ms > 0 {
                    size as u128 * 1000 / ms
                } else {
                    999999
                };
                println!(
                    "║  {:>5}  │  {:>7} ms │  {:>7} ms │  {:>7} ms │  {:>12} rows/s  ║",
                    size,
                    parse.as_millis(),
                    calc.as_millis(),
                    ms,
                    rows_per_sec
                );
            }
            Err(e) => println!("║  {:>5}  │  ERROR: {:50} ║", size, e),
        }
    }

    println!("╚═══════════════════════════════════════════════════════════════════════╝");
    println!();
}

#[test]
#[ignore]
fn perf_extended_report() {
    println!("\n");
    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║              FORGE EXTENDED PERFORMANCE REPORT                        ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║  Rows   │   Parse    │    Calc    │   Total    │   Rows/sec           ║");
    println!("╠═════════╪════════════╪════════════╪════════════╪══════════════════════╣");

    let sizes = [1000, 5000, 10000, 25000, 50000, 100000];
    for &size in &sizes {
        match bench_calculate(size) {
            Ok((parse, calc)) => {
                let total = parse + calc;
                let ms = total.as_millis();
                let rows_per_sec = if ms > 0 {
                    size as u128 * 1000 / ms
                } else {
                    999999
                };
                println!(
                    "║  {:>5}  │  {:>7} ms │  {:>7} ms │  {:>7} ms │  {:>12} rows/s  ║",
                    size,
                    parse.as_millis(),
                    calc.as_millis(),
                    ms,
                    rows_per_sec
                );
            }
            Err(e) => println!("║  {:>5}  │  ERROR: {:50} ║", size, e),
        }
    }

    println!("╚═══════════════════════════════════════════════════════════════════════╝");
    println!();
}

// ═══════════════════════════════════════════════════════════════════════════════
// MONTE CARLO PERFORMANCE BENCHMARKS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "full")]
mod monte_carlo_bench {
    use royalbit_forge::monte_carlo::{Distribution, MonteCarloConfig, MonteCarloEngine};
    use std::time::Instant;

    fn bench_monte_carlo(iterations: usize) -> std::time::Duration {
        let config = MonteCarloConfig {
            enabled: true,
            iterations,
            sampling: "latin_hypercube".to_string(),
            seed: Some(42),
            outputs: vec![royalbit_forge::monte_carlo::config::OutputConfig {
                variable: "revenue".to_string(),
                percentiles: vec![10, 50, 90],
                threshold: Some("> 100000".to_string()),
                label: None,
            }],
            correlations: vec![],
        };

        let mut engine = MonteCarloEngine::new(config).unwrap();
        engine.add_distribution("revenue", Distribution::normal(100000.0, 15000.0).unwrap());
        engine.add_distribution(
            "costs",
            Distribution::triangular(50000.0, 60000.0, 80000.0).unwrap(),
        );
        engine.add_distribution("margin", Distribution::uniform(0.1, 0.3).unwrap());

        let start = Instant::now();
        let _result = engine.run().unwrap();
        start.elapsed()
    }

    #[test]
    #[ignore]
    fn perf_monte_carlo_report() {
        println!("\n");
        println!("╔═══════════════════════════════════════════════════════════════════════╗");
        println!("║              MONTE CARLO PERFORMANCE REPORT                           ║");
        println!("╠═══════════════════════════════════════════════════════════════════════╣");
        println!("║  Iterations  │   Time (ms)  │  Iterations/sec                         ║");
        println!("╠══════════════╪══════════════╪═════════════════════════════════════════╣");

        let sizes = [1000, 5000, 10000, 25000, 50000, 100000];
        for &size in &sizes {
            let duration = bench_monte_carlo(size);
            let ms = duration.as_millis();
            let per_sec = if ms > 0 {
                size as u128 * 1000 / ms
            } else {
                999999
            };
            println!(
                "║  {:>10}  │  {:>10} ms │  {:>15} iter/s                   ║",
                size, ms, per_sec
            );
        }

        println!("╚═══════════════════════════════════════════════════════════════════════╝");
        println!();
    }

    #[test]
    fn perf_monte_carlo_10k_under_1_second() {
        // Target: 10,000 iterations should complete in under 1 second
        let duration = bench_monte_carlo(10000);
        assert!(
            duration.as_secs() < 1,
            "10K iterations took {} ms, should be under 1000 ms",
            duration.as_millis()
        );
    }

    #[test]
    fn perf_monte_carlo_1k_under_100_ms() {
        // Target: 1,000 iterations should complete in under 100 ms
        let duration = bench_monte_carlo(1000);
        assert!(
            duration.as_millis() < 100,
            "1K iterations took {} ms, should be under 100 ms",
            duration.as_millis()
        );
    }
}
