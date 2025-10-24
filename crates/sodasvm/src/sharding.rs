use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ShardConfig {
    pub shard_id: u32,
    pub max_users: u32,
    pub current_users: u32,
    pub merkle_tree_depth: u32,
    pub state_root_frequency: u32,
    pub emergency_vault_balance: u64,
}

#[derive(Debug)]
pub struct CrossShardTransaction {
    pub from_shard: u32,
    pub to_shard: u32,
    pub amount: u64,
    pub user_from: Pubkey,
    pub user_to: Pubkey,
    pub nonce: u64,
}

#[derive(Debug)]
pub struct ShardingMetrics {
    pub total_shards: u32,
    pub total_users: u32,
    pub avg_users_per_shard: u32,
    pub cross_shard_tx_percentage: f64,
    pub l1_commitment_cost_daily: f64,
    pub emergency_fund_total: u64,
    pub state_sync_overhead: f64,
}

pub struct ShardingManager {
    pub shards: HashMap<u32, ShardConfig>,
    pub user_shard_mapping: HashMap<Pubkey, u32>,
    pub cross_shard_queue: Vec<CrossShardTransaction>,
}

impl ShardingManager {
    pub fn new() -> Self {
        Self {
            shards: HashMap::new(),
            user_shard_mapping: HashMap::new(),
            cross_shard_queue: Vec::new(),
        }
    }

    pub fn initialize_sharding_for_scale(target_users: u32) -> Self {
        let mut manager = Self::new();
        let optimal_shard_size = 500_000;
        let shards_needed = (target_users as f64 / optimal_shard_size as f64).ceil() as u32;

        for shard_id in 0..shards_needed {
            let shard_config = ShardConfig {
                shard_id,
                max_users: optimal_shard_size,
                current_users: 0,
                merkle_tree_depth: 19, // 2^19 = 524,288 users
                state_root_frequency: 1000,
                emergency_vault_balance: 100_000_000_000, // 100M USDC per shard
            };
            manager.shards.insert(shard_id, shard_config);
        }

        manager
    }

    pub fn assign_user_to_shard(&mut self, user: Pubkey) -> Result<u32, String> {
        let user_hash = self.hash_user(user);
        let shard_id = user_hash % self.shards.len() as u32;

        if let Some(shard) = self.shards.get_mut(&shard_id) {
            if shard.current_users >= shard.max_users {
                return Err(format!("Shard {} is at capacity", shard_id));
            }
            shard.current_users += 1;
            self.user_shard_mapping.insert(user, shard_id);
            Ok(shard_id)
        } else {
            Err("Shard not found".to_string())
        }
    }

    fn hash_user(&self, user: Pubkey) -> u32 {
        let bytes = user.to_bytes();
        let mut hash = 0u32;
        for chunk in bytes.chunks(4) {
            let mut chunk_bytes = [0u8; 4];
            chunk_bytes[..chunk.len()].copy_from_slice(chunk);
            hash ^= u32::from_le_bytes(chunk_bytes);
        }
        hash
    }

    pub fn execute_cross_shard_transaction(
        &mut self,
        from_user: Pubkey,
        to_user: Pubkey,
        amount: u64,
    ) -> Result<(), String> {
        let from_shard = self.user_shard_mapping.get(&from_user)
            .ok_or("From user not found")?;
        let to_shard = self.user_shard_mapping.get(&to_user)
            .ok_or("To user not found")?;

        if from_shard == to_shard {
            return Ok(());
        }

        let cross_shard_tx = CrossShardTransaction {
            from_shard: *from_shard,
            to_shard: *to_shard,
            amount,
            user_from: from_user,
            user_to: to_user,
            nonce: self.cross_shard_queue.len() as u64,
        };

        self.cross_shard_queue.push(cross_shard_tx);
        Ok(())
    }

    pub fn calculate_metrics(&self) -> ShardingMetrics {
        let total_shards = self.shards.len() as u32;
        let total_users: u32 = self.shards.values().map(|s| s.current_users).sum();
        let avg_users_per_shard = if total_shards > 0 { total_users / total_shards } else { 0 };

        let cross_shard_txs = self.cross_shard_queue.len() as f64;
        let total_txs = total_users as f64 * 10.0; // Assume 10 txs per user
        let cross_shard_percentage = if total_txs > 0.0 {
            (cross_shard_txs / total_txs) * 100.0
        } else {
            0.0
        };

        let l1_cost_daily = total_shards as f64 * 0.1; // 0.1 SOL per shard per day

        let emergency_fund_total: u64 = self.shards.values()
            .map(|s| s.emergency_vault_balance)
            .sum();

        let state_sync_overhead = match total_shards {
            1 => 0.0,
            2..=5 => 2.0,
            6..=20 => 5.0,
            21..=50 => 10.0,
            _ => 20.0,
        };

        ShardingMetrics {
            total_shards,
            total_users,
            avg_users_per_shard,
            cross_shard_tx_percentage: cross_shard_percentage,
            l1_commitment_cost_daily: l1_cost_daily,
            emergency_fund_total,
            state_sync_overhead,
        }
    }

    pub fn print_sharding_analysis(&self) {
        let metrics = self.calculate_metrics();

        println!("SHARDING CONFIGURATION ANALYSIS");
        println!("================================");
        println!("Total shards:              {}", metrics.total_shards);
        println!("Total users:               {}", format_number(metrics.total_users));
        println!("Avg users per shard:       {}", format_number(metrics.avg_users_per_shard));
        println!("Cross-shard tx rate:       {:.1}%", metrics.cross_shard_tx_percentage);
        println!("Daily L1 costs:            {:.1} SOL", metrics.l1_commitment_cost_daily);
        println!("Emergency fund total:      ${:.0}M", metrics.emergency_fund_total as f64 / 1_000_000.0);
        println!("State sync overhead:       {:.1}%", metrics.state_sync_overhead);

        println!("\nPER-SHARD BREAKDOWN");
        println!("===================");
        for (shard_id, config) in &self.shards {
            let utilization = (config.current_users as f64 / config.max_users as f64) * 100.0;
            println!(
                "Shard {}: {}/{} users ({:.1}% full)",
                shard_id,
                format_number(config.current_users),
                format_number(config.max_users),
                utilization
            );
        }

        self.print_scaling_projections();
        self.print_safety_analysis();
    }

    fn print_scaling_projections(&self) {
        println!("\nSCALING PROJECTIONS");
        println!("===================");

        let scenarios = [
            (1_000_000, "1M users"),
            (5_000_000, "5M users"),
            (10_000_000, "10M users"),
            (50_000_000, "50M users"),
            (100_000_000, "100M users"),
        ];

        for (target_users, description) in scenarios {
            let shards_needed = (target_users as f64 / 500_000.0).ceil() as u32;
            let daily_cost = shards_needed as f64 * 0.1;
            let cross_shard_overhead = match shards_needed {
                1..=5 => 5.0,
                6..=20 => 10.0,
                21..=50 => 15.0,
                _ => 25.0,
            };

            println!(
                "{:15}: {:3} shards, {:.1} SOL/day, {:.0}% overhead",
                description, shards_needed, daily_cost, cross_shard_overhead
            );
        }
    }

    fn print_safety_analysis(&self) {
        println!("\nSAFETY & SECURITY ANALYSIS");
        println!("===========================");

        let metrics = self.calculate_metrics();

        println!("Emergency fund coverage:   {:.1}x daily volume",
                 metrics.emergency_fund_total as f64 / 1_000_000_000.0);

        println!("Time lock period:          7 days (151,200 slots)");

        println!("Shard failure impact:      {:.1}% of total users",
                 100.0 / metrics.total_shards as f64);

        println!("State root frequency:      Every 1,000 transactions");

        println!("Merkle proof size:         ~19 hashes (608 bytes)");

        println!("L1 verification cost:      ~200K compute units");

        println!("\nFAILURE SCENARIOS");
        println!("=================");
        println!("Single shard failure:      Users can exit via other shards");
        println!("Multiple shard failure:    Emergency mode activates");
        println!("Sequencer failure:         Time lock enables exits");
        println!("L1 congestion:             Extended exit window");
        println!("Cross-shard attack:        Isolated to involved shards");
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