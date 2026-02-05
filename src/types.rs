//! Core types for the arbitrage bot

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Token types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenType {
    Yes,
    No,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenType::Yes => write!(f, "YES"),
            TokenType::No => write!(f, "NO"),
        }
    }
}

/// Trade direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Buy,
    Sell,
}

/// Arbitrage type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArbType {
    YesNoSimple,
    YesNoMulti,
    GraphArbitrage,
    StatisticalArb,
    MevExtraction,
}

/// MEV type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MevType {
    FrontRunning,
    SandwichAttack,
    BackRunning,
    EquilibriumManipulation,
}

/// Market data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub id: String,
    pub question: String,
    pub yes_price: f64,
    pub no_price: f64,
    pub yes_liquidity: f64,
    pub no_liquidity: f64,
    pub timestamp: DateTime<Utc>,
    pub volume_24h: f64,
}

impl Default for MarketData {
    fn default() -> Self {
        Self {
            id: String::new(),
            question: String::new(),
            yes_price: 0.5,
            no_price: 0.5,
            yes_liquidity: 0.0,
            no_liquidity: 0.0,
            timestamp: Utc::now(),
            volume_24h: 0.0,
        }
    }
}

impl MarketData {
    pub fn yes_no_arbitrage(&self) -> Option<f64> {
        let sum = self.yes_price + self.no_price;
        if sum < 1.0 {
            Some(1.0 - sum)
        } else {
            None
        }
    }
}

/// Arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub market_id: String,
    pub question: String,
    pub arb_type: ArbType,
    pub profit: f64,
    pub roi_pct: f64,
    pub confidence: f64,
    pub yes_price: f64,
    pub no_price: f64,
    pub sum_price: f64,
    pub liquidity: f64,
    pub timestamp: DateTime<Utc>,
    pub legs: Option<Vec<ArbitrageLeg>>,
    pub path: Option<Vec<String>>,
}

/// Arbitrage leg
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageLeg {
    pub market_id: String,
    pub token_type: TokenType,
    pub direction: Direction,
    pub price: f64,
    pub quantity: f64,
}

/// Trade execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    pub trade_id: String,
    pub market_id: String,
    pub arb_type: ArbType,
    pub legs: Vec<ArbitrageLeg>,
    pub total_investment: f64,
    pub expected_return: f64,
    pub actual_return: f64,
    pub profit: f64,
    pub roi_pct: f64,
    pub entry_time: DateTime<Utc>,
    pub exit_time: DateTime<Utc>,
    pub execution_time_ms: u64,
    pub slippage_pct: f64,
    pub gas_cost: f64,
    pub fees: f64,
}

/// MEV opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevOpportunity {
    pub opportunity_type: MevType,
    pub victim_transactions: Vec<String>,
    pub expected_profit: f64,
    pub gas_cost: f64,
    pub net_profit: f64,
}

/// Risk metrics
#[derive(Debug, Clone)]
pub struct RiskMetrics {
    pub var_95: f64,
    pub daily_loss_limit: f64,
    pub max_consecutive_losses: u32,
    pub max_drawdown: f64,
    pub current_drawdown: f64,
    pub sharpe_ratio: f64,
}

impl Default for RiskMetrics {
    fn default() -> Self {
        Self {
            var_95: 0.0,
            daily_loss_limit: 50.0,
            max_consecutive_losses: 5,
            max_drawdown: 0.15,
            current_drawdown: 0.0,
            sharpe_ratio: 0.0,
        }
    }
}

/// Q-Learning state
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct QState {
    pub price_trend: i8,
    pub arbitrage_available: i8,
    pub z_score_bucket: i8,
}

/// Q-Learning action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QAction {
    BuyYes,
    BuyNo,
    BuyBoth,
    Sell,
    Hold,
}

/// Q-Table entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QEntry {
    pub buy_yes_value: f64,
    pub buy_no_value: f64,
    pub buy_both_value: f64,
    pub sell_value: f64,
    pub hold_value: f64,
}

impl Default for QEntry {
    fn default() -> Self {
        Self {
            buy_yes_value: 0.0,
            buy_no_value: 0.0,
            buy_both_value: 0.0,
            sell_value: 0.0,
            hold_value: 0.0,
        }
    }
}

impl QEntry {
    pub fn get_value(&self, action: QAction) -> f64 {
        match action {
            QAction::BuyYes => self.buy_yes_value,
            QAction::BuyNo => self.buy_no_value,
            QAction::BuyBoth => self.buy_both_value,
            QAction::Sell => self.sell_value,
            QAction::Hold => self.hold_value,
        }
    }

    pub fn set_value(&mut self, action: QAction, value: f64) {
        match action {
            QAction::BuyYes => self.buy_yes_value = value,
            QAction::BuyNo => self.buy_no_value = value,
            QAction::BuyBoth => self.buy_both_value = value,
            QAction::Sell => self.sell_value = value,
            QAction::Hold => self.hold_value = value,
        }
    }

    pub fn best_action(&self) -> QAction {
        let values = [
            (QAction::BuyYes, self.buy_yes_value),
            (QAction::BuyNo, self.buy_no_value),
            (QAction::BuyBoth, self.buy_both_value),
            (QAction::Sell, self.sell_value),
            (QAction::Hold, self.hold_value),
        ];
        values.into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap().0
    }
}

/// VWAP execution plan
#[derive(Debug, Clone)]
pub struct VwapExecutionPlan {
    pub market_id: String,
    pub total_quantity: f64,
    pub slices: Vec<VwapSlice>,
    pub expected_price: f64,
    pub expected_slippage: f64,
}

/// VWAP slice
#[derive(Debug, Clone)]
pub struct VwapSlice {
    pub quantity: f64,
    pub target_time: DateTime<Utc>,
    pub limit_price: Option<f64>,
}

/// Bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub initial_capital: f64,
    pub min_profit_threshold: f64,
    pub risk_per_trade: f64,
    pub max_position_size: f64,
    pub api_base: String,
    pub ws_url: String,
    pub api_key: Option<String>,
    pub enable_mev: bool,
    pub max_execution_time_ms: u64,
    pub polling_interval_ms: u64,
    pub use_real_data: bool, // Abilita dati reali da Polymarket API
    pub polymarket_api_key: Option<String>, // Polymarket API Key
    pub polymarket_secret: Option<String>,   // Polymarket API Secret
    pub polymarket_passphrase: Option<String>, // Polymarket API Passphrase
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            initial_capital: 1000.0,
            min_profit_threshold: 0.01,
            risk_per_trade: 0.02,
            max_position_size: 100.0,
            api_base: "https://api.polymarket.com".to_string(),
            ws_url: "wss://api.polymarket.com/ws".to_string(),
            api_key: None,
            enable_mev: false,
            max_execution_time_ms: 5000,
            polling_interval_ms: 1000,
            use_real_data: false,
            polymarket_api_key: None,
            polymarket_secret: None,
            polymarket_passphrase: None,
        }
    }
}
