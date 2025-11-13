// Allow `cargo stylus export-abi` to generate a main function.
#![cfg_attr(not(feature = "export-abi"), no_main)]
extern crate alloc;

use alloc::string::String;
use alloy_primitives::{Address, U256};
use stylus_sdk::{msg, prelude::*};

// ----------------------
// Storage layout
// ----------------------

sol_storage! {
    #[entrypoint]
    pub struct ResolveWallet {
        // Per-user balances
        mapping(address => uint256) available_balance;
        mapping(address => uint256) staked_balance;
        mapping(address => uint256) earned_total;
        mapping(address => uint256) burned_total;

        // Per-user performance stats
        mapping(address => uint256) wins;
        mapping(address => uint256) losses;
        mapping(address => uint256) current_win_streak;
        mapping(address => uint256) longest_win_streak;

        // Global pools / counters
        uint256 charity_pool;
        uint256 next_goal_id;

        // Goal data
        mapping(uint256 => address) goal_owner;
        mapping(uint256 => string) goal_text;
        mapping(uint256 => string) goal_category;
        mapping(uint256 => string) goal_priority;
        mapping(uint256 => uint256) goal_stake;
        mapping(uint256 => string) goal_deadline;
        // 0 = pending, 1 = completed, 2 = missed
        mapping(uint256 => uint256) goal_status;
    }
}

// ----------------------
// Public functions
// ----------------------

#[public]
impl ResolveWallet {
    // -----------------------------------
    // 1. Credits: "fake money" management
    // -----------------------------------

    /// Give yourself some fake credits.
    /// In a real version you might restrict this or link to deposits.
    pub fn deposit_credits(&mut self, amount: U256) {
        let user = msg::sender();

        // available_balance[user] += amount;
        let mut available = self.available_balance.setter(user);
        let current = available.get();
        available.set(current + amount);

        // earned_total can be purely from wins, so we don't touch it here.
    }

    /// Withdraw (destroy) credits from your available balance.
    /// We just reduce your internal balance. Returns false if not enough.
    pub fn withdraw_credits(&mut self, amount: U256) -> bool {
        let user = msg::sender();
        let mut available = self.available_balance.setter(user);
        let current = available.get();

        if current < amount {
            // not enough credits
            return false;
        }

        available.set(current - amount);
        true
    }

    // -----------------------------------
    // 2. Create a goal + stake credits
    // -----------------------------------

    /// Create a new goal and lock some credits into it.
    /// Returns the new goal_id, or 0 if not enough balance.
    pub fn create_goal(
        &mut self,
        text: String,
        category: String,
        priority: String,
        stake: U256,
        deadline: String,
    ) -> U256 {
        let user = msg::sender();

        // Check if user has enough available balance
        let mut available = self.available_balance.setter(user);
        let current_available = available.get();
        if current_available < stake {
            // Not enough credits to stake. We return 0 as "failed".
            return U256::from(0u8);
        }

        // Decrease available balance and increase staked balance
        available.set(current_available - stake);

        let mut staked = self.staked_balance.setter(user);
        let current_staked = staked.get();
        staked.set(current_staked + stake);

        // Compute new goal id (start from 1 so 0 can mean "failed")
        let current_id = self.next_goal_id.get();
        let new_id = current_id + U256::from(1u8);
        self.next_goal_id.set(new_id);

        // Store goal data
        self.goal_owner.setter(new_id).set(user);                 // Address
        self.goal_text.setter(new_id).set_str(&text);             // String
        self.goal_category.setter(new_id).set_str(&category);     // String
        self.goal_priority.setter(new_id).set_str(&priority);     // String
        self.goal_stake.setter(new_id).set(stake);                // U256
        self.goal_deadline.setter(new_id).set_str(&deadline);     // String
        self.goal_status
            .setter(new_id)
            .set(U256::from(0u8));                                // 0 = pending

        new_id
    }

    // -----------------------------------
    // 3. Resolve a goal: win or lose
    // -----------------------------------

    /// Mark a goal as completed (win).
    /// Returns true if success, false if goal doesn't belong to caller or not pending.
    pub fn complete_goal(&mut self, goal_id: U256) -> bool {
        let user = msg::sender();

        // Check owner
        let owner = self.goal_owner.get(goal_id);
        if owner != user {
            return false;
        }

        // Check status is pending (0)
        let status = self.goal_status.get(goal_id);
        if status != U256::from(0u8) {
            return false;
        }

        // Read stake amount
        let stake_amount = self.goal_stake.get(goal_id);

        // Update goal status -> completed (1)
        self.goal_status.setter(goal_id).set(U256::from(1u8));

        // Move stake back from staked -> available
        {
            let mut staked = self.staked_balance.setter(user);
            let current_staked = staked.get();
            staked.set(current_staked - stake_amount);
        }

        {
            let mut available = self.available_balance.setter(user);
            let current_available = available.get();
            available.set(current_available + stake_amount);
        }

        // Update earned_total
        {
            let mut earned = self.earned_total.setter(user);
            let current = earned.get();
            earned.set(current + stake_amount);
        }

        // Update stats: wins, streaks
        {
            // wins++
            let mut wins = self.wins.setter(user);
            let current_wins = wins.get();
            wins.set(current_wins + U256::from(1u8));

            // current_win_streak++
            let mut streak = self.current_win_streak.setter(user);
            let current_streak = streak.get() + U256::from(1u8);
            streak.set(current_streak);

            // longest_win_streak = max(longest, current)
            let mut longest = self.longest_win_streak.setter(user);
            let longest_val = longest.get();
            if current_streak > longest_val {
                longest.set(current_streak);
            }
        }

        true
    }

    /// Mark a goal as missed (loss).
    /// Stake goes to global charity pool. Returns true if success.
    pub fn miss_goal(&mut self, goal_id: U256) -> bool {
        let user = msg::sender();

        // Check owner
        let owner = self.goal_owner.get(goal_id);
        if owner != user {
            return false;
        }

        // Check status is pending (0)
        let status = self.goal_status.get(goal_id);
        if status != U256::from(0u8) {
            return false;
        }

        let stake_amount = self.goal_stake.get(goal_id);

        // Update goal status -> missed (2)
        self.goal_status.setter(goal_id).set(U256::from(2u8));

        // Move stake from staked -> burned + charity_pool
        {
            let mut staked = self.staked_balance.setter(user);
            let current_staked = staked.get();
            staked.set(current_staked - stake_amount);
        }

        {
            let mut burned = self.burned_total.setter(user);
            let current = burned.get();
            burned.set(current + stake_amount);
        }

        {
            let current_pool = self.charity_pool.get();
            self.charity_pool.set(current_pool + stake_amount);
        }

        // Update stats: losses, reset streak
        {
            let mut losses = self.losses.setter(user);
            let current_losses = losses.get();
            losses.set(current_losses + U256::from(1u8));

            // reset current streak
            self.current_win_streak
                .setter(user)
                .set(U256::from(0u8));
        }

        true
    }

    // -----------------------------------
    // 4. Read-only helpers for the frontend
    // -----------------------------------

    /// Get wallet balances for a user:
    /// (available, staked, earned_total, burned_total)
    pub fn balances_of(&self, user: Address) -> (U256, U256, U256, U256) {
        let available = self.available_balance.get(user);
        let staked = self.staked_balance.get(user);
        let earned = self.earned_total.get(user);
        let burned = self.burned_total.get(user);

        (available, staked, earned, burned)
    }

    /// Get performance stats for a user:
    /// (wins, losses, current_win_streak, longest_win_streak)
    pub fn stats_of(&self, user: Address) -> (U256, U256, U256, U256) {
        let wins = self.wins.get(user);
        let losses = self.losses.get(user);
        let current_streak = self.current_win_streak.get(user);
        let longest_streak = self.longest_win_streak.get(user);

        (wins, losses, current_streak, longest_streak)
    }

    /// Get global charity pool amount.
    pub fn charity_pool_total(&self) -> U256 {
        self.charity_pool.get()
    }

    /// Get all data for one goal.
    /// Returns:
    /// (owner, text, category, priority, stake, deadline, status)
    pub fn get_goal(&self, id: U256) -> (Address, String, String, String, U256, String, U256) {
        let owner   = self.goal_owner.get(id);
        let stake   = self.goal_stake.get(id);
        let status  = self.goal_status.get(id);

        let text      = self.goal_text.getter(id).get_string();
        let category  = self.goal_category.getter(id).get_string();
        let priority  = self.goal_priority.getter(id).get_string();
        let deadline  = self.goal_deadline.getter(id).get_string();

        (owner, text, category, priority, stake, deadline, status)
    }
}
