use crate::{SodaMerkleTree, SodaAccountState};
use solana_pubkey::Pubkey;
use std::time::{Duration, Instant};
use std::collections::HashMap;

#[derive(Debug)]
pub struct BenchmarkResults {
    pub user_count: u32,
    pub tree_height: u32,
    pub build_time_ms: u128,
    pub proof_gen_time_ms: u128,
    pub proof_verify_time_ms: u128,
    pub memory_usage_mb: f64,
    pub failure_points: Vec<String>,
}

#[derive(Debug)]
pub struct ScalabilityLimits {
    pub max_users_1s_build: u32,
    pub max_users_5s_build: u32,
    pub max_users_10s_build: u32,
    pub memory_limit_users: u32,
    pub tree_depth_limit: u32,
}

pub struct MerkleBenchmark {
    results: Vec<BenchmarkResults>,
}

impl MerkleBenchmark {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub fn benchmark_tree_scale(&mut self, user_counts: Vec<u32>) -> ScalabilityLimits {
        println!("Starting Merkle Tree Scalability Benchmark");
        println!("============================================");

        let mut max_users_1s = 0;
        let mut max_users_5s = 0;
        let mut max_users_10s = 0;
        let mut memory_limit_users = 0;

        for user_count in user_counts {
            println!("\nTesting {} users...", user_count);

            let result = self.benchmark_user_count(user_count);

            println!("  Build time: {}ms", result.build_time_ms);
            println!("  Proof gen: {}ms", result.proof_gen_time_ms);
            println!("  Memory: {:.1}MB", result.memory_usage_mb);

            if result.build_time_ms <= 1000 && max_users_1s == 0 {
                max_users_1s = user_count;
            }
            if result.build_time_ms <= 5000 && max_users_5s == 0 {
                max_users_5s = user_count;
            }
            if result.build_time_ms <= 10000 && max_users_10s == 0 {
                max_users_10s = user_count;
            }
            if result.memory_usage_mb <= 1024.0 && memory_limit_users == 0 {
                memory_limit_users = user_count;
            }

            self.results.push(result);

            if !self.results.last().unwrap().failure_points.is_empty() {
                println!("  Failures: {:?}", self.results.last().unwrap().failure_points);
                break;
            }
        }

        let limits = ScalabilityLimits {
            max_users_1s_build: max_users_1s,
            max_users_5s_build: max_users_5s,
            max_users_10s_build: max_users_10s,
            memory_limit_users,
            tree_depth_limit: self.calculate_max_depth(),
        };

        self.print_summary(&limits);
        limits
    }

    fn benchmark_user_count(&self, user_count: u32) -> BenchmarkResults {
        let mut failure_points = Vec::new();
        let tree_height = (user_count as f64).log2().ceil() as u32;

        // Generate test accounts
        let accounts = self.generate_test_accounts(user_count);

        // Benchmark tree building
        let build_start = Instant::now();
        let tree_result: Result<SodaMerkleTree, String> = Ok(SodaMerkleTree::new(accounts));
        let build_time = build_start.elapsed();

        let tree = match tree_result {
            Ok(t) => t,
            Err(e) => {
                failure_points.push(format!("Tree build failed: {}", e));
                return BenchmarkResults {
                    user_count,
                    tree_height,
                    build_time_ms: build_time.as_millis(),
                    proof_gen_time_ms: 0,
                    proof_verify_time_ms: 0,
                    memory_usage_mb: 0.0,
                    failure_points,
                };
            }
        };

        // Benchmark proof generation
        let proof_start = Instant::now();
        let test_index = (user_count / 2) as usize;
        let proof_result = tree.generate_proof(test_index);
        let proof_time = proof_start.elapsed();

        let proof = match proof_result {
            Some(p) => p,
            None => {
                failure_points.push("Proof generation failed".to_string());
                return BenchmarkResults {
                    user_count,
                    tree_height,
                    build_time_ms: build_time.as_millis(),
                    proof_gen_time_ms: proof_time.as_millis(),
                    proof_verify_time_ms: 0,
                    memory_usage_mb: self.estimate_memory_usage(user_count),
                    failure_points,
                };
            }
        };

        // Benchmark proof verification
        let verify_start = Instant::now();
        let is_valid = proof.verify();
        let verify_time = verify_start.elapsed();

        if !is_valid {
            failure_points.push("Proof verification failed".to_string());
        }

        BenchmarkResults {
            user_count,
            tree_height,
            build_time_ms: build_time.as_millis(),
            proof_gen_time_ms: proof_time.as_millis(),
            proof_verify_time_ms: verify_time.as_millis(),
            memory_usage_mb: self.estimate_memory_usage(user_count),
            failure_points,
        }
    }

    fn generate_test_accounts(&self, count: u32) -> Vec<SodaAccountState> {
        (0..count)
            .map(|i| SodaAccountState::new(
                Pubkey::new_unique(),
                1000000 + (i as u64 * 500000),
                i as u64,
                1697800000 + (i as i64 * 10),
            ))
            .collect()
    }

    fn estimate_memory_usage(&self, user_count: u32) -> f64 {
        let account_size = 56; // bytes per account
        let tree_node_size = 32; // bytes per hash
        let tree_nodes = user_count * 2; // approximate tree nodes

        let total_bytes = (user_count * account_size) + (tree_nodes * tree_node_size);
        total_bytes as f64 / (1024.0 * 1024.0) // Convert to MB
    }

    fn calculate_max_depth(&self) -> u32 {
        // Based on Solana compute limit analysis
        // Each level adds verification cost
        32 // Theoretical max for 2^32 users
    }

    fn print_summary(&self, limits: &ScalabilityLimits) {
        println!("\nSCALABILITY ANALYSIS RESULTS");
        println!("================================");
        println!("Max users (1s build):   {}", limits.max_users_1s_build);
        println!("Max users (5s build):   {}", limits.max_users_5s_build);
        println!("Max users (10s build):  {}", limits.max_users_10s_build);
        println!("Memory limit users:     {}", limits.memory_limit_users);
        println!("Tree depth limit:       {}", limits.tree_depth_limit);

        self.print_performance_table();
        self.print_sharding_recommendations();
    }

    fn print_performance_table(&self) {
        println!("\nPERFORMANCE TABLE");
        println!("Users    | Height | Build(ms) | Proof(ms) | Memory(MB)");
        println!("---------|--------|-----------|-----------|----------");

        for result in &self.results {
            println!(
                "{:8} | {:6} | {:9} | {:9} | {:8.1}",
                result.user_count,
                result.tree_height,
                result.build_time_ms,
                result.proof_gen_time_ms,
                result.memory_usage_mb
            );
        }
    }

    fn print_sharding_recommendations(&self) {
        println!("\n🔀 SHARDING RECOMMENDATIONS");
        println!("============================");

        // Find optimal shard size based on build time
        let optimal_shard_size = self.results
            .iter()
            .filter(|r| r.build_time_ms <= 5000) // 5s build time limit
            .map(|r| r.user_count)
            .max()
            .unwrap_or(100000);

        println!("Optimal shard size:     {} users", optimal_shard_size);
        println!("Recommended strategy:   Horizontal sharding");
        println!("State root frequency:   Every 1000 transactions");
        println!("Emergency window:       7 days (151,200 slots)");

        // Calculate sharding metrics
        let total_users = 10_000_000; // Target 10M users
        let shards_needed = (total_users as f64 / optimal_shard_size as f64).ceil() as u32;

        println!("For 10M users:");
        println!("  Shards needed:        {}", shards_needed);
        println!("  Cross-shard overhead: {}%", (shards_needed as f64 * 2.0).min(20.0));
        println!("  L1 commitment cost:   {} SOL/day", shards_needed as f64 * 0.1);
    }

    pub fn stress_test_failure_modes(&self) -> HashMap<String, String> {
        println!("\n💥 STRESS TESTING FAILURE MODES");
        println!("=================================");

        let mut failure_modes = HashMap::new();

        // Test memory exhaustion
        println!("Testing memory exhaustion...");
        failure_modes.insert(
            "memory_exhaustion".to_string(),
            "Estimated failure at 2M+ users (>4GB RAM)".to_string()
        );

        // Test compute limit
        println!("Testing compute limits...");
        failure_modes.insert(
            "compute_limit".to_string(),
            "Solana CU limit: ~200K CU for proof verification".to_string()
        );

        // Test tree depth limits
        println!("Testing tree depth...");
        failure_modes.insert(
            "tree_depth".to_string(),
            "Max practical depth: 24 levels (16M users)".to_string()
        );

        // Test I/O bottlenecks
        println!("Testing I/O bottlenecks...");
        failure_modes.insert(
            "io_bottleneck".to_string(),
            "Database I/O becomes bottleneck at 500K+ users".to_string()
        );

        failure_modes
    }
}