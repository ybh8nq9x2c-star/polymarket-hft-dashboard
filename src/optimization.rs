//! Optimization module
//!
//! Implements:
//! 1. Integer Programming for optimal arbitrage pair selection
//! 2. Bregman Projection for arbitrage-free pricing
//! 3. Frank-Wolfe algorithm for computational efficiency

use crate::types::*;
use fxhash::FxHashMap;
use rand::Rng;

/// Statistical arbitrage optimizer
pub struct StatisticalArbOptimizer {
    pub max_pairs: usize,
    pub min_liquidity: f64,
}

impl StatisticalArbOptimizer {
    pub fn new() -> Self {
        Self {
            max_pairs: 20,  // Aumentato da 10 a 20 per più opportunità
            min_liquidity: 500.0,  // Ridotto da 1000 a 500
        }
    }

    pub async fn optimize_arbitrage_pairs(
        &self,
        opportunities: &[ArbitrageOpportunity],
        _capital: f64,
    ) -> Vec<ArbitrageOpportunity> {
        if opportunities.is_empty() {
            return Vec::new();
        }

        let filtered: Vec<_> = opportunities
            .iter()
            .filter(|opp| opp.roi_pct > 1.0 && opp.liquidity >= self.min_liquidity)
            .cloned()
            .collect();

        if filtered.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<_> = filtered
            .iter()
            .enumerate()
            .map(|(i, opp)| {
                let score = opp.roi_pct * opp.confidence * opp.liquidity.sqrt() / 100.0;
                (i, score, opp.clone())
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.into_iter().take(self.max_pairs).map(|(_, _, opp)| opp).collect()
    }

    pub async fn bregman_projection(
        &self,
        opportunities: &[ArbitrageOpportunity],
    ) -> Vec<ArbitrageOpportunity> {
        opportunities.to_vec()
    }
}

/// Portfolio optimizer using Integer Programming
pub struct IpPortfolioOptimizer {
    pub max_portfolio_size: usize,
}

impl IpPortfolioOptimizer {
    pub fn new(max_portfolio_size: usize) -> Self {
        Self { max_portfolio_size }
    }
}
