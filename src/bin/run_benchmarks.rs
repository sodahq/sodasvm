use sodasvm::{SodaMerkleTree, SodaAccountState};
use solana_pubkey::Pubkey;
use std::time::Instant;
use std::fs::File;
use std::io::Write;

fn main() {
    println!("MERKLE TREE PERFORMANCE BENCHMARK");
    println!("==================================");
    println!("");

    let user_counts = vec![1_000, 10_000, 100_000, 500_000, 1_000_000, 5_000_000, 10_000_000, 25_000_000, 50_000_000, 100_000_000];

    for user_count in user_counts {
        println!("Testing {} users:", user_count);

        // Generate test accounts
        let accounts: Vec<SodaAccountState> = (0..user_count)
            .map(|i| SodaAccountState::new(
                Pubkey::new_unique(),
                1000000 + (i * 500000),
                i,
                1697800000 + (i as i64 * 10),
            ))
            .collect();

        // Benchmark tree building
        let start = Instant::now();
        let tree = SodaMerkleTree::new(accounts);
        let build_time = start.elapsed();

        // Calculate tree structure metrics
        let actual_leaves = user_count;
        let tree_height = (user_count as f64).log2().ceil() as u32;
        let padded_leaves = 2_u32.pow(tree_height);
        let total_nodes = padded_leaves * 2 - 1;
        let internal_nodes = padded_leaves - 1;

        println!("  Tree build time: {:.2}ms", build_time.as_millis());
        println!("  Tree height: {} levels", tree_height);
        println!("  Actual leaves: {}", actual_leaves);
        println!("  Padded leaves: {}", padded_leaves);
        println!("  Total nodes: {}", total_nodes);
        println!("  Internal nodes: {}", internal_nodes);

        // Benchmark proof generation
        let test_index = (user_count / 2) as usize;
        let start = Instant::now();
        let proof = tree.generate_proof(test_index);
        let proof_time = start.elapsed();

        if let Some(proof) = proof {
            println!("  Proof gen time: {:.2}μs", proof_time.as_micros());

            // Analyze proof structure
            let proof_path_length = proof.proof.len();
            let proof_size_bytes = proof_path_length * 32; // Each hash is 32 bytes

            println!("  Proof path length: {} hashes", proof_path_length);
            println!("  Proof size: {} bytes", proof_size_bytes);

            // Benchmark proof verification
            let start = Instant::now();
            let is_valid = proof.verify();
            let verify_time = start.elapsed();

            println!("  Proof verify time: {:.2}μs", verify_time.as_micros());
            println!("  Proof valid: {}", is_valid);
        } else {
            println!("  Proof generation failed");
        }

        // Estimate memory usage
        let account_size = 56; // bytes per account
        let tree_node_size = 32; // bytes per hash
        let tree_nodes = user_count * 2; // approximate tree nodes
        let total_bytes = (user_count * account_size) + (tree_nodes * tree_node_size);
        let memory_mb = total_bytes as f64 / (1024.0 * 1024.0);

        println!("  Memory usage: {:.1} MB", memory_mb);
        println!("");
    }

    println!("SCALABILITY ANALYSIS");
    println!("===================");

    // Find limits
    let mut max_1s_build = 0;
    let mut max_5s_build = 0;

    for user_count in [1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000, 1_000_000, 2_000_000, 5_000_000, 10_000_000] {
        let accounts: Vec<SodaAccountState> = (0..user_count)
            .map(|i| SodaAccountState::new(
                Pubkey::new_unique(),
                1000000 + (i * 500000),
                i,
                1697800000 + (i as i64 * 10),
            ))
            .collect();

        let start = Instant::now();
        let _tree = SodaMerkleTree::new(accounts);
        let build_time = start.elapsed();

        if build_time.as_millis() <= 1000 {
            max_1s_build = user_count;
        }
        if build_time.as_millis() <= 5000 {
            max_5s_build = user_count;
        }

        println!("{} users: {:.0}ms build time", user_count, build_time.as_millis());
    }

    println!("");
    println!("RESULTS:");
    println!("Max users (1s build): {}", max_1s_build);
    println!("Max users (5s build): {}", max_5s_build);

    // Sharding calculations
    let optimal_shard_size = max_5s_build.max(100_000);
    let target_users = 100_000_000; // 100M users
    let shards_needed = target_users / optimal_shard_size;

    println!("");
    // Write comprehensive results to CSV
    let mut csv_file = File::create("comprehensive_benchmark.csv").expect("Could not create CSV file");
    writeln!(csv_file, "users,build_time_ms,proof_gen_us,proof_verify_us,memory_mb,tree_height,actual_leaves,padded_leaves,total_nodes,internal_nodes,proof_path_length,proof_size_bytes,shards_needed,shard_size,cross_shard_overhead_pct,l1_cost_sol_day,storage_gb,sharding_recommended,bottleneck_type,emergency_exit_time_hours").expect("Could not write CSV header");

    for user_count in [1_000, 10_000, 100_000, 500_000, 1_000_000, 5_000_000, 10_000_000, 25_000_000, 50_000_000, 100_000_000] {
        println!("Benchmarking {} users...", user_count);

        let accounts: Vec<SodaAccountState> = (0..user_count)
            .map(|i| SodaAccountState::new(
                Pubkey::new_unique(),
                1000000 + (i * 500000),
                i,
                1697800000 + (i as i64 * 10),
            ))
            .collect();

        let start = Instant::now();
        let tree = SodaMerkleTree::new(accounts);
        let build_time = start.elapsed().as_millis();

        let test_index = (user_count / 2) as usize;
        let start = Instant::now();
        let proof = tree.generate_proof(test_index);
        let proof_time = start.elapsed().as_micros();

        let (verify_time, proof_size) = if let Some(ref p) = proof {
            let start = Instant::now();
            p.verify();
            let verify_time = start.elapsed().as_micros();
            let proof_size = p.proof.len() * 32; // Each hash is 32 bytes
            (verify_time, proof_size)
        } else {
            (0, 0)
        };

        // Memory calculation
        let memory_mb = ((user_count * 56) + (user_count * 2 * 32)) as f64 / (1024.0 * 1024.0);

        // Tree depth
        let tree_depth = (user_count as f64).log2().ceil() as u32;

        // Sharding analysis
        let optimal_shard_size = if build_time <= 5000 { user_count } else { 2_000_000 };
        let shards_needed = (user_count as f64 / optimal_shard_size as f64).ceil() as u32;
        let cross_shard_overhead = match shards_needed {
            1 => 0.0,
            2..=10 => 5.0,
            11..=50 => 10.0,
            51..=100 => 15.0,
            _ => 25.0,
        };

        // L1 costs (state commits every 10 minutes)
        let l1_cost_per_day = shards_needed as f64 * 144.0 * 0.0001; // 144 commits per day, 0.0001 SOL each

        // Storage requirements
        let storage_gb = (user_count * 56) as f64 / (1024.0 * 1024.0 * 1024.0);

        // Sharding recommendation
        let sharding_recommended = if user_count <= 2_000_000 { "NO" } else { "YES" };

        // Bottleneck analysis
        let bottleneck_type = if memory_mb > 8000.0 {
            "MEMORY"
        } else if build_time > 10000 {
            "CPU"
        } else if user_count > 10_000_000 {
            "SCALE"
        } else {
            "NONE"
        };

        // Emergency exit time (7 days in hours)
        let emergency_exit_hours = 168.0;

        // Calculate tree structure for CSV
        let tree_height_csv = (user_count as f64).log2().ceil() as u32;
        let padded_leaves_csv = 2_u32.pow(tree_height_csv);
        let total_nodes_csv = padded_leaves_csv * 2 - 1;
        let internal_nodes_csv = padded_leaves_csv - 1;

        writeln!(csv_file, "{},{},{},{},{:.1},{},{},{},{},{},{},{},{},{},{:.1},{:.4},{:.2},{},{},{:.1}",
                user_count, build_time, proof_time, verify_time, memory_mb, tree_height_csv,
                user_count, padded_leaves_csv, total_nodes_csv, internal_nodes_csv,
                tree_height_csv, proof_size, shards_needed, optimal_shard_size, cross_shard_overhead,
                l1_cost_per_day, storage_gb, sharding_recommended, bottleneck_type, emergency_exit_hours)
            .expect("Could not write CSV row");
    }

    println!("Comprehensive results saved to comprehensive_benchmark.csv");
}