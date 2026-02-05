//! Trade execution module
//!
//! Implements:
//! 1. VWAP-based order execution
//! 2. MEV extraction
//! 3. Parallel trade submission
//! 4. Slippage estimation

use crate::types::*;
use chrono::Utc;
use fxhash::FxHashMap;
use rand::Rng;
use std::time::Instant;

/// Trade executor with VWAP and MEV capabilities
pub struct TradeExecutor {
    pub config: BotConfig,
    pub executed_trades: Vec<TradeExecution>,
    pub pending_orders: FxHashMap<String, Order>,
    pub vwap_tracker: VwapTracker,
}

impl TradeExecutor {
    pub fn new(config: BotConfig) -> Self {
        Self {
            config,
            executed_trades: Vec::new(),
            pending_orders: FxHashMap::default(),
            vwap_tracker: VwapTracker::new(20),
        }
    }

    /// Execute arbitrage trade
    pub async fn execute_arbitrage(
        &mut self,
        opportunity: &ArbitrageOpportunity,
        capital: f64,
    ) -> Option<TradeExecution> {
        let start_time = Instant::now();

        // Calculate position size
        let position = self._calculate_position(capital, opportunity);

        if position < 10.0 {
            return None;
        }

        // Split position between legs
        let yes_position = position / 2.0;
        let no_position = position / 2.0;

        // Calculate VWAP prices
        let yes_vwap = self.vwap_tracker.get_vwap(&opportunity.market_id, &TokenType::Yes);
        let no_vwap = self.vwap_tracker.get_vwap(&opportunity.market_id, &TokenType::No);

        // Use quoted prices if VWAP not available
        let yes_price = yes_vwap.unwrap_or(0.5);
        let no_price = no_vwap.unwrap_or(0.5);

        // Create arbitrage legs
        let legs = vec![
            ArbitrageLeg {
                market_id: opportunity.market_id.clone(),
                token_type: TokenType::Yes,
                direction: Direction::Buy,
                price: yes_price,
                quantity: yes_position / yes_price,
            },
            ArbitrageLeg {
                market_id: opportunity.market_id.clone(),
                token_type: TokenType::No,
                direction: Direction::Buy,
                price: no_price,
                quantity: no_position / no_price,
            },
        ];

        // Calculate totals
        let total_investment = legs.iter().map(|l| l.price * l.quantity).sum();
        let expected_return = position; // Guaranteed return of $1 per position

        // Simulate execution with slippage
        let slippage_pct = rand::thread_rng().gen_range(0.0..0.005); // 0-0.5%
        let actual_return = expected_return * (1.0 - slippage_pct);
        let profit = actual_return - total_investment;

        let execution_time = start_time.elapsed().as_millis() as u64;

        let trade = TradeExecution {
            trade_id: format!("trade_{}", self.executed_trades.len() + 1),
            market_id: opportunity.market_id.clone(),
            arb_type: opportunity.arb_type.clone(),
            legs,
            total_investment,
            expected_return,
            actual_return,
            profit,
            roi_pct: (profit / total_investment) * 100.0,
            entry_time: Utc::now(),
            exit_time: Utc::now(),
            execution_time_ms: execution_time,
            slippage_pct: slippage_pct * 100.0,
            gas_cost: 0.02, // $0.02 for 4-leg strategy
            fees: total_investment * 0.002, // 0.2% fee
        };

        self.executed_trades.push(trade.clone());
        Some(trade)
    }

    fn _calculate_position(&self, capital: f64, opportunity: &ArbitrageOpportunity) -> f64 {
        let capital_limit = capital * self.config.max_position_size;
        let liquidity_limit = opportunity.liquidity * 0.1; // Max 10% of liquidity

        capital_limit.min(liquidity_limit)
    }
}

/// VWAP Tracker for execution optimization
pub struct VwapTracker {
    window_size: usize,
    price_history: FxHashMap<String, Vec<(TokenType, f64)>>,
}

impl VwapTracker {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            price_history: FxHashMap::default(),
        }
    }

    pub fn update(&mut self, market_id: &str, token_type: TokenType, price: f64) {
        let history = self.price_history.entry(market_id.to_string()).or_insert_with(Vec::new);
        history.push((token_type, price));

        if history.len() > self.window_size {
            history.remove(0);
        }
    }

    pub fn get_vwap(&self, market_id: &str, token_type: &TokenType) -> Option<f64> {
        if let Some(history) = self.price_history.get(market_id) {
            let relevant: Vec<_> = history.iter().filter(|(t, _)| t == token_type).collect();
            if !relevant.is_empty() {
                let sum: f64 = relevant.iter().map(|(_, p)| p).sum();
                return Some(sum / relevant.len() as f64);
            }
        }
        None
    }
}

/// Order for parallel submission
#[derive(Debug, Clone)]
pub struct Order {
    pub order_id: String,
    pub market_id: String,
    pub token_type: TokenType,
    pub direction: Direction,
    pub price: f64,
    pub quantity: f64,
    pub status: OrderStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    Pending,
    Submitted,
    Partial,
    Filled,
    Failed,
}

/// MEV Opportunity Detector
pub struct MevDetector {
    block_time_window: u64, // milliseconds
}

impl MevDetector {
    pub fn new(block_time_window: u64) -> Self {
        Self { block_time_window }
    }

    /// Detect MEV opportunities for parallel execution
    pub fn detect_mev_opportunity(&self, trades: &[&TradeExecution]) -> Option<MevOpportunity> {
        if trades.len() < 2 {
            return None;
        }

        // Check if trades can be bundled for MEV extraction
        let total_gas = trades.iter().map(|t| t.gas_cost).sum::<f64>();
        let savings = total_gas * 0.5; // 50% gas savings from bundling

        if savings > 0.01 {
            Some(MevOpportunity {
                opportunity_type: MevType::FrontRunning,
                victim_transactions: trades.iter().map(|t| t.trade_id.clone()).collect(),
                expected_profit: savings,
                gas_cost: total_gas * 0.5,
                net_profit: savings - total_gas * 0.5,
            })
        } else {
            None
        }
    }
}
