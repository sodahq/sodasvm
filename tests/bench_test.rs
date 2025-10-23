use sodasvm::{MerkleBenchmark, ScalabilityLimits};

#[tokio::test]
async fn test_merkle_scalability_benchmark() {
    let mut benchmark = MerkleBenchmark::new();

    let user_counts = vec![
        1_000,
        10_000,
        50_000,
        100_000,
        250_000,
        500_000,
        1_000_000,
        2_000_000,
    ];

    let limits = benchmark.benchmark_tree_scale(user_counts);

    println!("1-second build limit:   {} users", limits.max_users_1s_build);
    println!("5-second build limit:   {} users", limits.max_users_5s_build);
    println!("10-second build limit:  {} users", limits.max_users_10s_build);
    println!("Memory limit:           {} users", limits.memory_limit_users);
    println!("Max tree depth:         {} levels", limits.tree_depth_limit);

    let failure_modes = benchmark.stress_test_failure_modes();

    for (mode, description) in failure_modes {
        println!("{}: {}", mode, description);
    }

    calculate_sharding_strategy(&limits);

    assert!(limits.max_users_1s_build >= 10_000);
    assert!(limits.max_users_5s_build >= 100_000);
    assert!(limits.memory_limit_users >= 100_000);
}

fn calculate_sharding_strategy(limits: &ScalabilityLimits) {
    let target_users = 10_000_000;
    let optimal_shard_size = limits.max_users_5s_build.max(100_000);

    let shards_needed = (target_users as f64 / optimal_shard_size as f64).ceil() as u32;
    let l1_cost_per_day = shards_needed as f64 * 0.1;

    println!("Target scale:             {} users", target_users);
    println!("Optimal shard size:       {} users", optimal_shard_size);
    println!("Shards required:          {}", shards_needed);
    println!("Daily L1 costs:           {:.1} SOL", l1_cost_per_day);
    println!("Cross-shard overhead:     {}%", calculate_cross_shard_overhead(shards_needed));

    let emergency_window_slots = 151_200;
    let slot_time_ms = 400;
    let emergency_window_hours = (emergency_window_slots * slot_time_ms as u64) / (1000 * 3600);

    println!("Time lock duration:       {} hours ({} days)", emergency_window_hours, emergency_window_hours / 24);
    println!("Slots per time lock:      {}", emergency_window_slots);

    let account_size_bytes = 56;
    let total_storage_gb = (target_users as f64 * account_size_bytes as f64) / (1024.0 * 1024.0 * 1024.0);

    println!("Account storage:          {:.1} GB", total_storage_gb);
    println!("Tree node storage:        {:.1} GB", total_storage_gb * 2.0);
    println!("Total storage:            {:.1} GB", total_storage_gb * 3.0);
    println!("Growth rate estimate:     ~{:.1} GB/year", total_storage_gb * 0.5);
}

fn calculate_cross_shard_overhead(shards: u32) -> u32 {
    match shards {
        1 => 0,
        2..=10 => 5,
        11..=50 => 10,
        51..=100 => 15,
        _ => 20,
    }
}

#[tokio::test]
async fn test_real_world_scenarios() {
    test_defi_scenario().await;
    test_gaming_scenario().await;
    test_payments_scenario().await;
}

async fn test_defi_scenario() {
    let mut benchmark = MerkleBenchmark::new();
    let result = benchmark.benchmark_tree_scale(vec![100_000]);

    println!("DeFi Protocol (100K users)");
    println!("  Transaction volume:     ~1,000 TPS");
    println!("  State commitment freq:  Every 1,000 txs");
    println!("  Emergency fund size:    $10M USDC");
    println!("  Time lock period:       7 days");
}

async fn test_gaming_scenario() {
    let mut benchmark = MerkleBenchmark::new();
    let result = benchmark.benchmark_tree_scale(vec![1_000_000]);

    println!("Gaming Platform (1M users)");
    println!("  Transaction volume:     ~10,000 TPS");
    println!("  State commitment freq:  Every 10,000 txs");
    println!("  Emergency fund size:    $100M USDC");
    println!("  Time lock period:       3 days");
}

async fn test_payments_scenario() {
    println!("Payments App (10M users)");
    println!("  Transaction volume:     ~100,000 TPS");
    println!("  State commitment freq:  Every 100,000 txs");
    println!("  Emergency fund size:    $1B USDC");
    println!("  Time lock period:       7 days");
    println!("  Sharding required:      100 shards");
}