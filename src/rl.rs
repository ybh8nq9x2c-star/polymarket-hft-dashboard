//! Reinforcement Learning module for adaptive trading
//!
//! Implements:
//! 1. Q-Learning for adaptive trade signals
//! 2. EMRT (Empirical Mean Reversion Time) for mean reversion detection
//! 3. Model-free RL framework

use crate::types::*;
use rand::Rng;
use std::collections::HashMap;

/// Q-Learning optimizer for adaptive trading signals
pub struct QLearningOptimizer {
    q_table: HashMap<String, HashMap<usize, f64>>,
    epsilon: f64,
    alpha: f64,
    gamma: f64,
}

impl QLearningOptimizer {
    pub fn new(epsilon: f64, alpha: f64, gamma: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            epsilon,
            alpha,
            gamma,
        }
    }

    /// Get state key from market conditions
    fn get_state_key(&self, z_score: f64, momentum: f64, arb_available: bool) -> String {
        let z_bucket = if z_score > 2.0 { "high" } else if z_score < -2.0 { "low" } else { "mid" };
        let m_bucket = if momentum > 0.01 { "up" } else if momentum < -0.01 { "down" } else { "flat" };
        let arb_str = if arb_available { "yes" } else { "no" };
        format!("{}_{}_{}", z_bucket, m_bucket, arb_str)
    }

    /// Epsilon-greedy action selection
    pub fn get_action(
        &mut self,
        z_score: f64,
        momentum: f64,
        arb_available: bool
    ) -> usize {
        let state = self.get_state_key(z_score, momentum, arb_available);

        // Initialize state if not exists
        self.q_table.entry(state.clone()).or_insert_with(|| {
            let mut actions = HashMap::new();
            for action in 0..3 {
                actions.insert(action, 0.0);
            }
            actions
        });

        let actions = self.q_table.get(&state).unwrap();

        // Epsilon-greedy: explore with probability epsilon
        if rand::thread_rng().gen::<f64>() < self.epsilon {
            return rand::thread_rng().gen_range(0..3);
        }

        // Exploit: choose best action
        let mut best_action = 0;
        let mut best_q = f64::NEG_INFINITY;

        for (&action, &q) in actions {
            if q > best_q {
                best_q = q;
                best_action = action;
            }
        }

        best_action
    }

    /// Update Q-value
    pub fn update(&mut self, z_score: f64, momentum: f64, arb_available: bool, action: usize, reward: f64) {
        let state = self.get_state_key(z_score, momentum, arb_available);

        if let Some(actions) = self.q_table.get_mut(&state) {
            let current_q = actions.get(&action).copied().unwrap_or(0.0);
            let max_future_q = actions.values().cloned().fold(f64::NEG_INFINITY, f64::max);

            let new_q = current_q + self.alpha * (reward + self.gamma * max_future_q - current_q);
            actions.insert(action, new_q);
        }
    }
}

/// EMRT (Empirical Mean Reversion Time) Calculator
pub struct EmrtCalculator {
    window: usize,
    threshold: f64,
}

impl EmrtCalculator {
    pub fn new(window: usize, threshold: f64) -> Self {
        Self { window, threshold }
    }

    /// Calculate EMRT for a price series
    pub fn calculate_emrt(&self, prices: &[f64]) -> f64 {
        if prices.len() < 2 {
            return 0.0;
        }

        let mut reversion_times = Vec::new();
        let mut current_trend_start = 0;
        let mut current_trend = if prices[1] > prices[0] { 1.0 } else { -1.0 };

        for i in 1..prices.len() {
            let trend = if prices[i] > prices[i-1] { 1.0 } else { -1.0 };

            if trend != current_trend {
                reversion_times.push(i - current_trend_start);
                current_trend_start = i;
                current_trend = trend;
            }
        }

        if reversion_times.is_empty() {
            return prices.len() as f64;
        }

        let sum: f64 = reversion_times.iter().map(|&x| x as f64).sum();
        sum / reversion_times.len() as f64
    }

    /// Find optimal hedge ratio for pair trading
    pub fn find_hedge_ratio(&self, asset1: &[f64], asset2: &[f64]) -> f64 {
        if asset1.len() != asset2.len() || asset1.len() < 10 {
            return 1.0;
        }

        let mut best_a = None;
        for i in -3i32..=3 {
            let a = i as f64;
            let spread: Vec<f64> = asset1.iter().zip(asset2.iter()).
                map(|(p1, p2)| a * p1 + p2).collect();
            let emrt = self.calculate_emrt(&spread);
            let current = (emrt, a);
            best_a = Some(best_a.map_or(current, |prev: (f64, f64)| if prev.0 < current.0 { prev } else { current }));
        }

        best_a.map(|(_, a)| a).unwrap_or(1.0)
    }
}

/// State representation for RL
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TradingState {
    pub price_z_score: i32,
    pub momentum_level: i32,
    pub arb_available: bool,
}

impl TradingState {
    pub fn from_prices(prices: &[f64], arb_available: bool) -> Self {
        if prices.len() < 2 {
            return Self {
                price_z_score: 0,
                momentum_level: 0,
                arb_available,
            };
        }

        let last_price = prices[prices.len() - 1];
        let prev_price = prices[prices.len() - 2];
        let mean = prices.iter().sum::<f64>() / prices.len() as f64;
        let std_dev = if prices.len() > 1 {
            let variance = prices.iter().map(|&p| (p - mean).powi(2)).sum::<f64>() / prices.len() as f64;
            variance.sqrt()
        } else {
            0.0
        };

        let z_score = if std_dev > 0.0 {
            (last_price - mean) / std_dev
        } else {
            0.0
        };

        let price_z_score = (z_score.clamp(-3.0, 3.0) as i32);

        let momentum = if prev_price > 0.0 {
            (last_price - prev_price) / prev_price
        } else {
            0.0
        };

        let momentum_level = (momentum.clamp(-0.02, 0.02) * 100.0) as i32;

        Self {
            price_z_score,
            momentum_level,
            arb_available,
        }
    }
}

/// Action representation for RL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TradingAction {
    BuyYes,
    BuyNo,
    NoTrade,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_q_learning() {
        let mut optimizer = QLearningOptimizer::new(0.1, 0.1, 0.95);

        let action = optimizer.get_action(0.5, 0.01, true);
        assert!(action < 3);

        optimizer.update(0.5, 0.01, true, action, 1.0);
    }

    #[test]
    fn test_emrt() {
        let calculator = EmrtCalculator::new(10, 0.01);
        let prices = vec![100.0, 101.0, 99.0, 100.0];
        let emrt = calculator.calculate_emrt(&prices);
        assert!(emrt > 0.0);
    }
}
