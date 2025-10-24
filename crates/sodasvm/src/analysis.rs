use std::collections::HashMap;

#[derive(Debug)]
pub struct TreeCapacityAnalysis {
    pub max_users_by_depth: HashMap<u32, u32>,
    pub memory_usage_by_depth: HashMap<u32, f64>,
    pub proof_size_by_depth: HashMap<u32, u32>,
    pub verification_cost_by_depth: HashMap<u32, u64>,
}

#[derive(Debug)]
pub struct FailurePoint {
    pub user_count: u32,
    pub failure_type: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug)]
pub struct ShardingAnalysis {
    pub optimal_shard_size: u32,
    pub shards_for_10m_users: u32,
    pub cross_shard_tx_overhead: f64,
    pub l1_commitment_cost_daily: f64,
    pub state_sync_complexity: String,
}

pub struct DepthAnalyzer;

impl DepthAnalyzer {
    pub fn analyze_tree_capacity() -> TreeCapacityAnalysis {
        let mut analysis = TreeCapacityAnalysis {
            max_users_by_depth: HashMap::new(),
            memory_usage_by_depth: HashMap::new(),
            proof_size_by_depth: HashMap::new(),
            verification_cost_by_depth: HashMap::new(),
        };

        for depth in 10..=30 {
            let max_users = 2_u32.pow(depth);
            let memory_mb = Self::calculate_memory_usage(max_users);
            let proof_size = depth as u32;
            let verification_cost = Self::calculate_verification_cost(depth);

            analysis.max_users_by_depth.insert(depth, max_users);
            analysis.memory_usage_by_depth.insert(depth, memory_mb);
            analysis.proof_size_by_depth.insert(depth, proof_size);
            analysis.verification_cost_by_depth.insert(depth, verification_cost);
        }

        analysis
    }

    fn calculate_memory_usage(user_count: u32) -> f64 {
        let account_size = 56.0;
        let tree_node_size = 32.0;
        let tree_nodes = user_count as f64 * 2.0;

        let total_bytes = (user_count as f64 * account_size) + (tree_nodes * tree_node_size);
        total_bytes / (1024.0 * 1024.0)
    }

    fn calculate_verification_cost(depth: u32) -> u64 {
        let base_cost = 10000_u64;
        let per_level_cost = 5000_u64;

        base_cost + (depth as u64 * per_level_cost)
    }

    pub fn identify_failure_points() -> Vec<FailurePoint> {
        vec![
            FailurePoint {
                user_count: 1_048_576, // 2^20
                failure_type: "Memory Pressure".to_string(),
                description: "Tree operations become memory-bound".to_string(),
                severity: "Medium".to_string(),
            },
            FailurePoint {
                user_count: 4_194_304, // 2^22
                failure_type: "I/O Bottleneck".to_string(),
                description: "Database operations saturate disk I/O".to_string(),
                severity: "High".to_string(),
            },
            FailurePoint {
                user_count: 16_777_216, // 2^24
                failure_type: "Compute Limit".to_string(),
                description: "Proof generation exceeds reasonable time limits".to_string(),
                severity: "Critical".to_string(),
            },
            FailurePoint {
                user_count: 33_554_432, // 2^25
                failure_type: "Solana CU Limit".to_string(),
                description: "Proof verification exceeds Solana compute units".to_string(),
                severity: "Critical".to_string(),
            },
            FailurePoint {
                user_count: 1_073_741_824, // 2^30
                failure_type: "Theoretical Limit".to_string(),
                description: "Maximum tree depth for practical systems".to_string(),
                severity: "Absolute".to_string(),
            },
        ]
    }

    pub fn analyze_sharding_requirements() -> ShardingAnalysis {
        let optimal_shard_size = 500_000;
        let target_users = 10_000_000;
        let shards_needed = (target_users as f64 / optimal_shard_size as f64).ceil() as u32;

        let cross_shard_overhead = match shards_needed {
            1..=10 => 0.05,
            11..=50 => 0.10,
            51..=100 => 0.15,
            _ => 0.20,
        };

        let l1_cost_daily = shards_needed as f64 * 0.1;

        let state_sync_complexity = match shards_needed {
            1..=5 => "Simple",
            6..=20 => "Moderate",
            21..=50 => "Complex",
            _ => "Very Complex",
        }.to_string();

        ShardingAnalysis {
            optimal_shard_size,
            shards_for_10m_users: shards_needed,
            cross_shard_tx_overhead: cross_shard_overhead,
            l1_commitment_cost_daily: l1_cost_daily,
            state_sync_complexity,
        }
    }

    pub fn print_analysis() {
        println!("TREE DEPTH CAPACITY ANALYSIS");
        println!("=============================");

        let capacity = Self::analyze_tree_capacity();

        println!("Depth | Max Users    | Memory(GB) | Proof Size | Verification Cost");
        println!("------|--------------|------------|------------|------------------");

        for depth in [16, 20, 24, 28, 30] {
            if let (Some(&users), Some(&memory), Some(&proof_size), Some(&cost)) = (
                capacity.max_users_by_depth.get(&depth),
                capacity.memory_usage_by_depth.get(&depth),
                capacity.proof_size_by_depth.get(&depth),
                capacity.verification_cost_by_depth.get(&depth),
            ) {
                println!(
                    "{:5} | {:12} | {:10.1} | {:10} | {:16}",
                    depth,
                    format_number(users),
                    memory / 1024.0,
                    proof_size,
                    cost
                );
            }
        }

        println!("\nFAILURE POINT ANALYSIS");
        println!("======================");

        let failure_points = Self::identify_failure_points();
        for failure in failure_points {
            println!(
                "{}: {} users - {} ({})",
                failure.severity,
                format_number(failure.user_count),
                failure.failure_type,
                failure.description
            );
        }

        println!("\nSHARDING ANALYSIS");
        println!("=================");

        let sharding = Self::analyze_sharding_requirements();
        println!("Optimal shard size:         {} users", format_number(sharding.optimal_shard_size));
        println!("Shards for 10M users:      {}", sharding.shards_for_10m_users);
        println!("Cross-shard overhead:       {:.1}%", sharding.cross_shard_tx_overhead * 100.0);
        println!("Daily L1 commitment cost:   {:.1} SOL", sharding.l1_commitment_cost_daily);
        println!("State sync complexity:      {}", sharding.state_sync_complexity);

        Self::print_real_world_limits();
    }

    fn print_real_world_limits() {
        println!("\nREAL-WORLD PERFORMANCE LIMITS");
        println!("==============================");

        let scenarios = [
            ("Single shard", 500_000, "5s build time, 1GB memory"),
            ("Light sharding (5 shards)", 2_500_000, "Acceptable cross-shard overhead"),
            ("Medium sharding (20 shards)", 10_000_000, "Complex but manageable"),
            ("Heavy sharding (100 shards)", 50_000_000, "Very complex, high overhead"),
        ];

        for (name, users, description) in scenarios {
            println!("{:25}: {:10} users - {}", name, format_number(users), description);
        }

        println!("\nRECOMMENDATE LIMITS");
        println!("===================");
        println!("Conservative:               500K users per shard");
        println!("Aggressive:                 1M users per shard");
        println!("Theoretical maximum:        16M users per shard");
        println!("Practical production:       2-5M users total");
    }
}

fn format_number(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}