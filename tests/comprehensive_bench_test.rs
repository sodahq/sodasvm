use sodasvm::{ComprehensiveBenchmark, BenchmarkConfig};

#[test]
fn test_comprehensive_svm_benchmark() {
    println!("COMPREHENSIVE SVM PERFORMANCE ANALYSIS");
    println!("======================================");
    println!("");

    let config = BenchmarkConfig {
        iterations: 50,
        warmup_iterations: 5,
        user_counts: vec![
            1_000,
            10_000,
            100_000,
        ],
        max_memory_gb: 8.0,
        target_build_time_ms: 5000,
        target_proof_time_ms: 100,
    };

    let mut benchmark = ComprehensiveBenchmark::new(config);
    let (sharding_analysis, bottlenecks) = benchmark.run_comprehensive_benchmark();

    println!("\nSHARDING ANALYSIS RESULTS");
    println!("=========================");
    println!("Optimal shard size:           {} users", sharding_analysis.optimal_shard_size);
    println!("Shards needed (200M users):   {}", sharding_analysis.shards_needed);
    println!("Cross-shard overhead:         {:.1}%", sharding_analysis.cross_shard_overhead_percent);
    println!("L1 commitment cost:           {:.2} SOL/day", sharding_analysis.l1_commitment_cost_per_day_sol);
    println!("Storage requirements:         {:.1} GB", sharding_analysis.storage_requirements_gb);
    println!("Network bandwidth:            {:.1} Mbps", sharding_analysis.network_bandwidth_mbps);

    println!("\nSCALABILITY BOTTLENECKS");
    println!("=======================");
    println!("Memory exhaustion:            {} users", format_user_count(bottlenecks.memory_exhaustion_point));
    println!("CPU saturation:               {} users", format_user_count(bottlenecks.cpu_saturation_point));

    assert!(sharding_analysis.optimal_shard_size >= 100_000, "Shard size too small");
}

fn format_user_count(count: u64) -> String {
    if count == u64::MAX {
        "No limit found".to_string()
    } else if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}