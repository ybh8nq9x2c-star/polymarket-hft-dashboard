//! Polymarket Ultra-High-Frequency Arbitrage Bot
//!
//! A sophisticated trading bot implementing advanced algorithms:
//! - YES/NO Arbitrage Detection
//! - Integer Programming for Optimization
//! - Bregman Projection with Frank-Wolfe
//! - Q-Learning for Adaptive Trading
//! - Modified Moore-Bellman-Ford for Graph Arbitrage
//! - VWAP Execution Strategy
//! - MEV Extraction
//! - Advanced Risk Management (VaR, Sharpe, Drawdown)

pub mod types;
pub mod arbitrage;
pub mod optimization;
pub mod rl;
pub mod execution;
pub mod market;
pub mod risk;
pub mod polymarket_api;

pub mod api_server;

pub use types::*;
pub use arbitrage::*;
pub use optimization::*;
pub use rl::*;
pub use execution::*;
pub use market::*;
pub use risk::*;
pub use polymarket_api::*;

/// Main orchestrator for the HFT arbitrage bot
pub struct HftArbitrageBot {
    pub config: BotConfig,
    pub arb_detector: ArbitrageDetector,
    pub graph_detector: GraphArbitrageDetector,
    pub optimizer: StatisticalArbOptimizer,
    pub portfolio_optimizer: IpPortfolioOptimizer,
    pub rl_agent: QLearningOptimizer,
    pub executor: TradeExecutor,
    pub mev_extractor: MevDetector,
    pub market_manager: MarketManager,
    pub risk_manager: RiskManager,
    pub position_sizer: PositionSizer,
    pub polymarket_api: Option<PolymarketApiClient>, // API client per dati reali
    pub capital: f64,
    pub initial_capital: f64,
    pub current_step: u64,
}

impl HftArbitrageBot {
    pub fn new(config: BotConfig) -> Self {
        let initial_capital = config.initial_capital;
        
        Self {
            config: config.clone(),
            arb_detector: ArbitrageDetector::new(
                config.min_profit_threshold,
                1000.0,
            ),
            graph_detector: GraphArbitrageDetector::new(),
            optimizer: StatisticalArbOptimizer::new(),
            portfolio_optimizer: IpPortfolioOptimizer::new(10),
            rl_agent: QLearningOptimizer::new(0.1, 0.95, 0.1),
            executor: TradeExecutor::new(config.clone()),
            mev_extractor: if config.enable_mev { MevDetector::new(1000) } else { MevDetector::new(0) },
            market_manager: MarketManager::new(1000.0, 50),
            risk_manager: RiskManager::new(50.0, 10, 0.15, 0.10, 0.20, 10),
            position_sizer: PositionSizer::new(0.25, 0.05, 10.0),
            polymarket_api: if config.use_real_data {
                Some(PolymarketApiClient::new(
                    PolymarketApiConfig::default(),
                    config.polymarket_api_key.clone(),
                    config.polymarket_secret.clone(),
                    config.polymarket_passphrase.clone(),
                ))
            } else {
                None
            },
            capital: initial_capital,
            initial_capital,
            current_step: 0,
        }
    }

    /// Run a single trading step
    pub async fn run_step(&mut self) -> Result<StepResult, String> {
        self.current_step += 1;
        
        // Update market prices
        self.market_manager.update_prices().await?;
        
        // Get all markets
        let markets: Vec<_> = self.market_manager.markets.values().cloned().collect();
        
        // Detect arbitrage opportunities
        let simple_arbs = self.arb_detector.scan_markets(&markets);
        let graph_arbs = self.graph_detector.detect_arbitrage_cycles();
        let mut all_opportunities = simple_arbs;
        all_opportunities.extend(graph_arbs);
        
        if all_opportunities.is_empty() {
            return Ok(StepResult {
                step: self.current_step,
                opportunities: 0,
                trades: 0,
                profit: 0.0,
                capital: self.capital,
                win_rate: 0.0,
            });
        }
        
        // Optimize using Integer Programming
        let optimized = self.optimizer
            .optimize_arbitrage_pairs(&all_opportunities, self.capital)
            .await;
        
        // Apply Bregman projection
        let projected = self.optimizer
            .bregman_projection(&optimized)
            .await;

        // Logging dettagliato per debugging
        eprintln!("Step {}: {} opportunità trovate, {} dopo optimizer, {} dopo Bregman projection",
            self.current_step,
            all_opportunities.len(),
            optimized.len(),
            projected.len()
        );
        
        if projected.is_empty() {
            return Ok(StepResult {
                step: self.current_step,
                opportunities: all_opportunities.len(),
                trades: 0,
                profit: 0.0,
                capital: self.capital,
                win_rate: 0.0,
            });
        }
        
        // Check risk controls
        if !self.risk_manager.can_trade(self.capital) {
            return Ok(StepResult {
                step: self.current_step,
                opportunities: all_opportunities.len(),
                trades: 0,
                profit: 0.0,
                capital: self.capital,
                win_rate: 0.0,
            });
        }
        
        // Execute top opportunity
        let trade: Option<TradeExecution> = self.executor
            .execute_arbitrage(&projected[0], self.capital)
            .await;
        
        let profit = trade.as_ref().map(|t| t.profit).unwrap_or(0.0);
        self.capital += profit;
        
        // Update risk metrics
        self.risk_manager.update(profit, self.capital);
        
        // Update Q-Learning
        if let Some(ref t) = trade {
            let reward = if t.profit > 0.0 { 1.0 } else { -1.0 };
            // Get state and action (simplified)
            let state = QState {
                price_trend: 0,
                arbitrage_available: 1,
                z_score_bucket: 0,
            };
            let next_state = state;
            // Update Q-learning with individual parameters
            let opportunity = &projected[0];
            let z_score = if (1.0 - opportunity.sum_price) > 0.02 { 2.5 } else { 0.5 };
            let momentum = 0.01; // Simplified
            let arb_available = true;
            let action = self.rl_agent.get_action(z_score, momentum, arb_available);
            self.rl_agent.update(z_score, momentum, arb_available, action, reward);
        }
        
        let trades = if trade.is_some() { 1 } else { 0 };
        
        Ok(StepResult {
            step: self.current_step,
            opportunities: all_opportunities.len(),
            trades,
            profit,
            capital: self.capital,
            win_rate: self.executor.executed_trades.len() as f64,
        })
    }

    /// Run simulation for multiple steps
    pub async fn run_simulation(&mut self, num_steps: u64) -> SimulationResult {
        let mut results = Vec::new();
        
        // Initialize markets
        self.market_manager.fetch_markets().await.unwrap();
        
        for _ in 0..num_steps {
            match self.run_step().await {
                Ok(result) => results.push(result),
                Err(e) => eprintln!("Step error: {}", e),
            }
        }
        
        let total_profit = self.capital - self.initial_capital;
        let total_trades = self.executor.executed_trades.len();
        let successful = self.executor
            .executed_trades
            .iter()
            .filter(|t| t.profit > 0.0)
            .count();
        let win_rate = if total_trades > 0 {
            successful as f64 / total_trades as f64
        } else {
            0.0
        };
        
        SimulationResult {
            num_steps,
            initial_capital: self.initial_capital,
            final_capital: self.capital,
            total_profit,
            total_roi: (total_profit / self.initial_capital) * 100.0,
            total_trades,
            successful_trades: successful,
            win_rate,
            steps: results,
        }
    }
}

pub struct StepResult {
    pub step: u64,
    pub opportunities: usize,
    pub trades: u32,
    pub profit: f64,
    pub capital: f64,
    pub win_rate: f64,
}

pub struct SimulationResult {
    pub num_steps: u64,
    pub initial_capital: f64,
    pub final_capital: f64,
    pub total_profit: f64,
    pub total_roi: f64,
    pub total_trades: usize,
    pub successful_trades: usize,
    pub win_rate: f64,
    pub steps: Vec<StepResult>,
}


// API Server exports for dashboard
pub use api_server::{
    start_api_server, AppState, BotState, SimulatedTrade, MarketInfo,
    LiveData, ArbitrageOpportunity, BotControlRequest, ApiResponse
};
