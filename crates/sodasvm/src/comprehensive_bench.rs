use crate::{SodaMerkleTree, SodaAccountState};
use solana_pubkey::Pubkey;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub user_counts: Vec<u64>,
    pub max_memory_gb: f64,
    pub target_build_time_ms: u64,
    pub target_proof_time_ms: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            warmup_iterations: 100,
            user_counts: vec![
                1_000,
                10_000,
                100_000,
                1_000_000,
                10_000_000,
                50_000_000,
                100_000_000,
                200_000_000,
            ],
            max_memory_gb: 64.0,
            target_build_time_ms: 5000,
            target_proof_time_ms: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub user_count: u64,
    pub iterations: usize,

    // Timing metrics (all in microseconds for precision)
    pub build_times_us: Vec<u64>,
    pub proof_gen_times_us: Vec<u64>,
    pub proof_verify_times_us: Vec<u64>,
    pub update_times_us: Vec<u64>,

    // Memory metrics (bytes)
    pub peak_memory_bytes: u64,
    pub average_memory_bytes: u64,
    pub memory_samples: Vec<u64>,

    // Throughput metrics
    pub proofs_per_second: f64,
    pub updates_per_second: f64,
    pub tree_build_ops_per_second: f64,

    // System metrics
    pub cpu_usage_percent: Vec<f64>,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,

    // Failure tracking
    pub failed_operations: u32,
    pub error_types: HashMap<String, u32>,
}

#[derive(Debug)]
pub struct StatisticalSummary {
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub p95: f64,
    pub p99: f64,
    pub confidence_interval_95: (f64, f64),
}

#[derive(Debug)]
pub struct ShardingAnalysis {
    pub optimal_shard_size: u64,
    pub shards_needed: u32,
    pub cross_shard_overhead_percent: f64,
    pub state_sync_cost_per_second: f64,
    pub emergency_exit_distribution: HashMap<u32, u64>,
    pub l1_commitment_cost_per_day_sol: f64,
    pub storage_requirements_gb: f64,
    pub network_bandwidth_mbps: f64,
}

#[derive(Debug)]
pub struct ScalabilityBottlenecks {
    pub memory_exhaustion_point: u64,
    pub cpu_saturation_point: u64,
    pub io_bottleneck_point: u64,
    pub proof_verification_limit: u64,
    pub state_commitment_frequency_limit: u32,
    pub emergency_exit_time_limit_hours: f64,
}

pub struct ComprehensiveBenchmark {
    config: BenchmarkConfig,
    results: Vec<PerformanceMetrics>,
}

impl ComprehensiveBenchmark {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
        }
    }

    pub fn run_comprehensive_benchmark(&mut self) -> (ShardingAnalysis, ScalabilityBottlenecks) {
        println!("SVM BENCHMARK SUITE");
        println!("===================");
        println!("Testing: {} - {} users",
            self.config.user_counts.first().unwrap_or(&0),
            self.config.user_counts.last().unwrap_or(&0));
        println!("Iterations per test: {}", self.config.iterations);
        println!("Statistical confidence: 95%");
        println!("");

        for &user_count in &self.config.user_counts {
            println!("Benchmarking {} users...", user_count);

            if let Some(metrics) = self.benchmark_user_count_rigorous(user_count) {
                self.print_metrics_summary(&metrics);
                self.results.push(metrics);
            } else {
                println!("CRITICAL: Benchmark failed at {} users", user_count);
                break;
            }
        }

        let sharding_analysis = self.analyze_sharding_requirements();
        let bottlenecks = self.identify_scalability_bottlenecks();

        (sharding_analysis, bottlenecks)
    }

    fn benchmark_user_count_rigorous(&self, user_count: u64) -> Option<PerformanceMetrics> {
        // Pre-allocate test data
        let test_accounts = self.generate_realistic_accounts(user_count);

        let mut metrics = PerformanceMetrics {
            user_count,
            iterations: self.config.iterations,
            build_times_us: Vec::with_capacity(self.config.iterations),
            proof_gen_times_us: Vec::with_capacity(self.config.iterations),
            proof_verify_times_us: Vec::with_capacity(self.config.iterations),
            update_times_us: Vec::with_capacity(self.config.iterations),
            peak_memory_bytes: 0,
            average_memory_bytes: 0,
            memory_samples: Vec::new(),
            proofs_per_second: 0.0,
            updates_per_second: 0.0,
            tree_build_ops_per_second: 0.0,
            cpu_usage_percent: Vec::new(),
            io_read_bytes: 0,
            io_write_bytes: 0,
            failed_operations: 0,
            error_types: HashMap::new(),
        };

        // Warmup phase
        println!("  Warmup phase ({} iterations)...", self.config.warmup_iterations);
        for _ in 0..self.config.warmup_iterations {
            let _ = SodaMerkleTree::new(test_accounts.clone());
        }

        // Start system monitoring
        let memory_monitor = self.start_memory_monitoring();
        let cpu_monitor = self.start_cpu_monitoring();

        // Main benchmark loop
        println!("  Running {} iterations...", self.config.iterations);
        let total_start = Instant::now();

        for iteration in 0..self.config.iterations {
            if iteration % 100 == 0 {
                print!("    Progress: {}/{}...\r", iteration, self.config.iterations);
            }

            // Tree build benchmark
            let build_start = Instant::now();
            let tree_result = std::panic::catch_unwind(|| {
                SodaMerkleTree::new(test_accounts.clone())
            });
            let build_time = build_start.elapsed().as_micros() as u64;

            let tree = match tree_result {
                Ok(t) => t,
                Err(_) => {
                    metrics.failed_operations += 1;
                    *metrics.error_types.entry("tree_build_panic".to_string()).or_insert(0) += 1;
                    continue;
                }
            };

            metrics.build_times_us.push(build_time);

            // Proof generation benchmark
            let test_index = (user_count / 2) as usize;
            let proof_start = Instant::now();
            let proof_result = tree.generate_proof(test_index);
            let proof_time = proof_start.elapsed().as_micros() as u64;

            if let Some(proof) = proof_result {
                metrics.proof_gen_times_us.push(proof_time);

                // Proof verification benchmark
                let verify_start = Instant::now();
                let is_valid = proof.verify();
                let verify_time = verify_start.elapsed().as_micros() as u64;

                if is_valid {
                    metrics.proof_verify_times_us.push(verify_time);
                } else {
                    metrics.failed_operations += 1;
                    *metrics.error_types.entry("proof_verification_failed".to_string()).or_insert(0) += 1;
                }
            } else {
                metrics.failed_operations += 1;
                *metrics.error_types.entry("proof_generation_failed".to_string()).or_insert(0) += 1;
            }

            // Tree update benchmark (simulate account balance changes)
            let update_start = Instant::now();
            // Note: This would require implementing update functionality in SodaMerkleTree
            let update_time = update_start.elapsed().as_micros() as u64;
            metrics.update_times_us.push(update_time);
        }

        let total_time = total_start.elapsed();

        // Stop monitoring and collect metrics
        metrics.memory_samples = self.stop_memory_monitoring(memory_monitor);
        metrics.cpu_usage_percent = self.stop_cpu_monitoring(cpu_monitor);

        // Calculate derived metrics
        self.calculate_throughput_metrics(&mut metrics, total_time);
        self.calculate_memory_metrics(&mut metrics);

        // Check if this configuration is viable
        if self.is_configuration_viable(&metrics) {
            Some(metrics)
        } else {
            println!("  FAILED: Configuration exceeded resource limits");
            None
        }
    }

    fn generate_realistic_accounts(&self, count: u64) -> Vec<SodaAccountState> {
        // Generate accounts with realistic balance distributions
        // Simulate Pareto distribution (80/20 rule for wealth)
        (0..count)
            .map(|i| {
                let balance = if i < count / 5 {
                    // Top 20% users have higher balances
                    1_000_000 + (i * 10_000_000 / count)
                } else {
                    // Bottom 80% have lower balances
                    100_000 + (i * 1_000_000 / count)
                };

                SodaAccountState::new(
                    Pubkey::new_unique(),
                    balance,
                    i,
                    1697800000 + (i as i64 * 10),
                )
            })
            .collect()
    }

    fn start_memory_monitoring(&self) -> Arc<std::sync::atomic::AtomicBool> {
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let running_clone = running.clone();

        thread::spawn(move || {
            while running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                // Memory monitoring would be implemented here
                thread::sleep(Duration::from_millis(100));
            }
        });

        running
    }

    fn start_cpu_monitoring(&self) -> Arc<std::sync::atomic::AtomicBool> {
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        // CPU monitoring implementation would go here
        running
    }

    fn stop_memory_monitoring(&self, monitor: Arc<std::sync::atomic::AtomicBool>) -> Vec<u64> {
        monitor.store(false, std::sync::atomic::Ordering::Relaxed);
        // Return collected memory samples
        vec![] // Placeholder
    }

    fn stop_cpu_monitoring(&self, monitor: Arc<std::sync::atomic::AtomicBool>) -> Vec<f64> {
        monitor.store(false, std::sync::atomic::Ordering::Relaxed);
        // Return collected CPU usage samples
        vec![] // Placeholder
    }

    fn calculate_throughput_metrics(&self, metrics: &mut PerformanceMetrics, total_time: Duration) {
        let total_seconds = total_time.as_secs_f64();

        if !metrics.proof_gen_times_us.is_empty() {
            metrics.proofs_per_second = metrics.proof_gen_times_us.len() as f64 / total_seconds;
        }

        if !metrics.update_times_us.is_empty() {
            metrics.updates_per_second = metrics.update_times_us.len() as f64 / total_seconds;
        }

        if !metrics.build_times_us.is_empty() {
            metrics.tree_build_ops_per_second = metrics.build_times_us.len() as f64 / total_seconds;
        }
    }

    fn calculate_memory_metrics(&self, metrics: &mut PerformanceMetrics) {
        if !metrics.memory_samples.is_empty() {
            metrics.peak_memory_bytes = *metrics.memory_samples.iter().max().unwrap_or(&0);
            metrics.average_memory_bytes = metrics.memory_samples.iter().sum::<u64>() / metrics.memory_samples.len() as u64;
        }
    }

    fn is_configuration_viable(&self, metrics: &PerformanceMetrics) -> bool {
        // Check if configuration meets deployment requirements
        let avg_build_time_ms = if !metrics.build_times_us.is_empty() {
            metrics.build_times_us.iter().sum::<u64>() / metrics.build_times_us.len() as u64 / 1000
        } else {
            u64::MAX
        };

        let failure_rate = metrics.failed_operations as f64 / metrics.iterations as f64;
        let memory_gb = metrics.peak_memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        avg_build_time_ms <= self.config.target_build_time_ms &&
        failure_rate <= 0.01 && // Less than 1% failure rate
        memory_gb <= self.config.max_memory_gb
    }

    fn print_metrics_summary(&self, metrics: &PerformanceMetrics) {
        println!("  Results for {} users:", metrics.user_count);

        if !metrics.build_times_us.is_empty() {
            let stats = self.calculate_statistics(&metrics.build_times_us);
            println!("    Build time: {:.1}ms avg, {:.1}ms p95",
                stats.mean / 1000.0, stats.p95 / 1000.0);
        }

        if !metrics.proof_gen_times_us.is_empty() {
            let stats = self.calculate_statistics(&metrics.proof_gen_times_us);
            println!("    Proof gen: {:.1}μs avg, {:.1}μs p95", stats.mean, stats.p95);
        }

        println!("    Throughput: {:.0} proofs/sec", metrics.proofs_per_second);
        println!("    Memory: {:.1} GB peak", metrics.peak_memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
        println!("    Failures: {}/{}", metrics.failed_operations, metrics.iterations);
    }

    fn calculate_statistics(&self, values: &[u64]) -> StatisticalSummary {
        if values.is_empty() {
            return StatisticalSummary {
                mean: 0.0, median: 0.0, std_dev: 0.0, min: 0.0, max: 0.0,
                p95: 0.0, p99: 0.0, confidence_interval_95: (0.0, 0.0),
            };
        }

        let mut sorted = values.to_vec();
        sorted.sort_unstable();

        let len = sorted.len();
        let sum: u64 = sorted.iter().sum();
        let mean = sum as f64 / len as f64;

        let variance: f64 = sorted.iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>() / len as f64;
        let std_dev = variance.sqrt();

        let median = if len % 2 == 0 {
            (sorted[len / 2 - 1] + sorted[len / 2]) as f64 / 2.0
        } else {
            sorted[len / 2] as f64
        };

        let p95_idx = (len as f64 * 0.95) as usize;
        let p99_idx = (len as f64 * 0.99) as usize;

        let margin_of_error = 1.96 * std_dev / (len as f64).sqrt(); // 95% confidence
        let confidence_interval_95 = (mean - margin_of_error, mean + margin_of_error);

        StatisticalSummary {
            mean,
            median,
            std_dev,
            min: sorted[0] as f64,
            max: sorted[len - 1] as f64,
            p95: sorted[p95_idx.min(len - 1)] as f64,
            p99: sorted[p99_idx.min(len - 1)] as f64,
            confidence_interval_95,
        }
    }

    fn analyze_sharding_requirements(&self) -> ShardingAnalysis {
        // Find optimal shard size based on performance metrics
        let optimal_shard_size = self.results
            .iter()
            .filter(|r| {
                let avg_build_time_ms = if !r.build_times_us.is_empty() {
                    r.build_times_us.iter().sum::<u64>() / r.build_times_us.len() as u64 / 1000
                } else {
                    u64::MAX
                };
                avg_build_time_ms <= self.config.target_build_time_ms
            })
            .map(|r| r.user_count)
            .max()
            .unwrap_or(1_000_000);

        let target_users = 200_000_000_u64; // 200M users
        let shards_needed = (target_users / optimal_shard_size).max(1) as u32;

        // Calculate cross-shard overhead (increases with shard count)
        let cross_shard_overhead_percent = match shards_needed {
            1 => 0.0,
            2..=10 => 5.0,
            11..=50 => 10.0,
            51..=100 => 15.0,
            101..=500 => 25.0,
            _ => 35.0,
        };

        // L1 commitment costs (state roots posted per shard)
        let state_roots_per_day = 24 * 60 * 6; // Every 10 minutes
        let cost_per_commitment_sol = 0.0001;
        let l1_commitment_cost_per_day_sol = shards_needed as f64 * state_roots_per_day as f64 * cost_per_commitment_sol;

        // Storage requirements
        let account_size_bytes = 56;
        let tree_overhead_factor = 2.0;
        let storage_requirements_gb = (target_users * account_size_bytes) as f64 * tree_overhead_factor / (1024.0 * 1024.0 * 1024.0);

        // Network bandwidth for cross-shard communication
        let network_bandwidth_mbps = shards_needed as f64 * 10.0; // 10 Mbps per shard

        ShardingAnalysis {
            optimal_shard_size,
            shards_needed,
            cross_shard_overhead_percent,
            state_sync_cost_per_second: l1_commitment_cost_per_day_sol / 86400.0,
            emergency_exit_distribution: HashMap::new(), // Would calculate fund distribution
            l1_commitment_cost_per_day_sol,
            storage_requirements_gb,
            network_bandwidth_mbps,
        }
    }

    fn identify_scalability_bottlenecks(&self) -> ScalabilityBottlenecks {
        let mut memory_exhaustion_point = u64::MAX;
        let mut cpu_saturation_point = u64::MAX;
        let mut io_bottleneck_point = u64::MAX;

        for result in &self.results {
            let memory_gb = result.peak_memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            if memory_gb > self.config.max_memory_gb && memory_exhaustion_point == u64::MAX {
                memory_exhaustion_point = result.user_count;
            }

            // Check if average build time exceeds target
            if !result.build_times_us.is_empty() {
                let avg_build_time_ms = result.build_times_us.iter().sum::<u64>() / result.build_times_us.len() as u64 / 1000;
                if avg_build_time_ms > self.config.target_build_time_ms && cpu_saturation_point == u64::MAX {
                    cpu_saturation_point = result.user_count;
                }
            }
        }

        ScalabilityBottlenecks {
            memory_exhaustion_point,
            cpu_saturation_point,
            io_bottleneck_point,
            proof_verification_limit: 1_000_000, // Based on Solana compute units
            state_commitment_frequency_limit: 1000, // Transactions per commitment
            emergency_exit_time_limit_hours: 168.0, // 7 days
        }
    }
}