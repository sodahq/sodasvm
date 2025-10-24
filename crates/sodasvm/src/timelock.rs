
#[derive(Debug, Clone)]
pub struct TimeLock {
    pub duration_slots: u64,
    pub last_state_root_slot: u64,
    pub current_slot: u64,
}

impl TimeLock {
    pub fn new(duration_days: u32) -> Self {
        let slot_time_ms = 400; // 400ms per slot
        let duration_slots = (duration_days as u64 * 24 * 60 * 60 * 1000) / slot_time_ms;

        Self {
            duration_slots,
            last_state_root_slot: 0,
            current_slot: 1000, // Start at slot 1000
        }
    }

    pub fn seven_day_lock() -> Self {
        Self::new(7)
    }

    pub fn update_state_root_posted(&mut self) {
        self.last_state_root_slot = self.current_slot;
        println!("State root posted at slot {}", self.current_slot);
    }

    pub fn advance_slots(&mut self, slots: u64) {
        self.current_slot += slots;
        println!("Advanced to slot {} (+{} slots)", self.current_slot, slots);
    }

    pub fn simulate_time_passage(&mut self, days: u32) {
        let slot_time_ms = 400;
        let slots_to_advance = (days as u64 * 24 * 60 * 60 * 1000) / slot_time_ms;
        self.advance_slots(slots_to_advance);
        println!("Simulated {} days passage ({} slots)", days, slots_to_advance);
    }

    pub fn is_emergency_withdrawal_allowed(&self) -> bool {
        let slots_elapsed = self.current_slot.saturating_sub(self.last_state_root_slot);
        slots_elapsed >= self.duration_slots
    }

    pub fn slots_until_emergency_allowed(&self) -> u64 {
        let slots_elapsed = self.current_slot.saturating_sub(self.last_state_root_slot);
        if slots_elapsed >= self.duration_slots {
            0
        } else {
            self.duration_slots - slots_elapsed
        }
    }

    pub fn time_until_emergency_allowed(&self) -> String {
        let slots_remaining = self.slots_until_emergency_allowed();
        if slots_remaining == 0 {
            return "Emergency withdrawals ALLOWED".to_string();
        }

        let slot_time_ms = 400;
        let ms_remaining = slots_remaining * slot_time_ms;
        let hours_remaining = ms_remaining / (1000 * 60 * 60);
        let days_remaining = hours_remaining / 24;

        if days_remaining > 0 {
            format!("{} days, {} hours remaining", days_remaining, hours_remaining % 24)
        } else {
            format!("{} hours remaining", hours_remaining)
        }
    }

    pub fn print_status(&self) {
        println!("\nTIME LOCK STATUS");
        println!("================");
        println!("Current slot:           {}", self.current_slot);
        println!("Last state root slot:   {}", self.last_state_root_slot);
        println!("Duration (slots):       {}", self.duration_slots);
        println!("Slots elapsed:          {}",
                 self.current_slot.saturating_sub(self.last_state_root_slot));
        println!("Emergency withdrawals:  {}",
                 if self.is_emergency_withdrawal_allowed() { "ALLOWED" } else { "BLOCKED" });
        println!("Time remaining:         {}", self.time_until_emergency_allowed());
    }
}