//! Market data management module
//!
//! Implements:
//! 1. Market data fetching and updates
//! 2. Price tracking and caching
//! 3. Liquidity monitoring
//! 4. WebSocket connection for real-time data

use crate::types::*;
use fxhash::FxHashMap;
use rand::Rng;

/// Market manager
pub struct MarketManager {
    pub markets: FxHashMap<String, MarketData>,
    pub price_history: FxHashMap<String, Vec<PriceSnapshot>>,
    pub config: MarketConfig,
    pub websocket_connected: bool,
}

impl MarketManager {
    pub fn new(min_liquidity: f64, max_markets: usize) -> Self {
        Self {
            markets: FxHashMap::default(),
            price_history: FxHashMap::default(),
            config: MarketConfig {
                min_liquidity,
                max_markets,
                update_interval_ms: 1000,
            },
            websocket_connected: false,
        }
    }

    /// Fetch markets from Polymarket API
    pub async fn fetch_markets(&mut self) -> Result<(), String> {
        // Simulate fetching markets
        let num_markets = self.config.max_markets.min(50);
        
        for i in 0..num_markets {
            let market = self._generate_simulated_market(i);
            self.add_market(market);
        }
        
        Ok(())
    }

    /// Add market to cache
    pub fn add_market(&mut self, market: MarketData) {
        let market_id = market.id.clone();
        
        // Add price snapshot to history
        let snapshot = PriceSnapshot {
            timestamp: market.timestamp,
            yes_price: market.yes_price,
            no_price: market.no_price,
            volume: market.volume_24h,
        };
        
        self.price_history
            .entry(market_id.clone())
            .or_insert_with(Vec::new)
            .push(snapshot);
        
        // Keep only last 1000 snapshots
        if let Some(history) = self.price_history.get_mut(&market_id) {
            if history.len() > 1000 {
                history.remove(0);
            }
        }
        
        self.markets.insert(market_id, market);
    }

    /// Update market prices
    pub async fn update_prices(&mut self) -> Result<(), String> {
        let mut rng = rand::thread_rng();
        
        for market in self.markets.values_mut() {
            // Simulate price movement - update independently to preserve arbitrage
            let yes_change = rng.gen_range(-0.02..0.02); // -2% to +2%
            let no_change = rng.gen_range(-0.02..0.02);  // -2% to +2%

            market.yes_price = (market.yes_price * (1.0 + yes_change)).max(0.01).min(0.99);
            market.no_price = (market.no_price * (1.0 + no_change)).max(0.01).min(0.99);

            // Occasionally create new arbitrage opportunities (10% chance per step)
            if rng.gen_bool(0.10) {
                let mispricing = rng.gen_range(0.01..0.05); // 1-5% mispricing
                market.yes_price = (market.yes_price - mispricing * 0.5).max(0.01);
                market.no_price = (market.no_price - mispricing * 0.5).max(0.01);
            }
            
            // Update liquidity and volume
            market.yes_liquidity = market.yes_liquidity * rng.gen_range(0.95..1.05);
            market.no_liquidity = market.no_liquidity * rng.gen_range(0.95..1.05);
            market.volume_24h = market.volume_24h * rng.gen_range(0.99..1.01);
            
            // Update timestamp
            market.timestamp = chrono::Utc::now();
            
            // Add to price history
            let snapshot = PriceSnapshot {
                timestamp: market.timestamp,
                yes_price: market.yes_price,
                no_price: market.no_price,
                volume: market.volume_24h,
            };
            
            self.price_history
                .entry(market.id.clone())
                .or_insert_with(Vec::new)
                .push(snapshot);
            
            // Keep history bounded
            if let Some(history) = self.price_history.get_mut(&market.id) {
                if history.len() > 1000 {
                    history.remove(0);
                }
            }
        }
        
        Ok(())
    }

    /// Get market by ID
    pub fn get_market(&self, market_id: &str) -> Option<&MarketData> {
        self.markets.get(market_id)
    }

    /// Get all markets
    pub fn get_all_markets(&self) -> Vec<&MarketData> {
        self.markets.values().collect()
    }

    /// Get markets with minimum liquidity
    pub fn get_liquid_markets(&self, min_liquidity: f64) -> Vec<&MarketData> {
        self.markets
            .values()
            .filter(|m| m.yes_liquidity + m.no_liquidity >= min_liquidity)
            .collect()
    }

    /// Get price history for market
    pub fn get_price_history(&self, market_id: &str) -> Vec<PriceSnapshot> {
        self.price_history
            .get(market_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Connect to WebSocket for real-time data
    pub async fn connect_websocket(&mut self) -> Result<(), String> {
        // Simulate WebSocket connection
        self.websocket_connected = true;
        Ok(())
    }

    /// Disconnect WebSocket
    pub fn disconnect_websocket(&mut self) {
        self.websocket_connected = false;
    }

    /// Generate simulated market for testing
    fn _generate_simulated_market(&self, index: usize) -> MarketData {
        let mut rng = rand::thread_rng();
        
        let questions = [
            "Will BTC exceed $100k by end of year?",
            "Will ETH flip BTC market cap?",
            "Will SOL reach $500?",
            "Will AVAX staking APY exceed 15%?",
            "Will DOT governance proposal pass?",
            "Will LINK oracle integration complete?",
            "Will MATIC achieve 100k TPS?",
            "Will UNI v4 launch this quarter?",
            "Will AAVE deploy on new chain?",
            "Will SUSHI governance token burn occur?",
        ];
        
        let yes_price = rng.gen_range(0.3..0.7);
        let no_price = 1.0 - yes_price;
        
        // 30% chance to create arbitrage opportunity
        let (yes_price, no_price) = if rng.gen_bool(0.3) {
            let mispricing = rng.gen_range(0.01..0.05);
            let adjusted_yes: f64 = yes_price - mispricing * 0.5;
            let adjusted_no: f64 = no_price - mispricing * 0.5;
            (adjusted_yes.max(0.01), adjusted_no.max(0.01))
        } else {
            (yes_price, no_price)
        };
        
        MarketData {
            id: format!("market_{}", index),
            question: questions[index % questions.len()].to_string(),
            yes_price,
            no_price,
            yes_liquidity: rng.gen_range(5000.0..50000.0),
            no_liquidity: rng.gen_range(5000.0..50000.0),
            volume_24h: rng.gen_range(10000.0..100000.0),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Market configuration
#[derive(Debug, Clone)]
pub struct MarketConfig {
    pub min_liquidity: f64,
    pub max_markets: usize,
    pub update_interval_ms: u64,
}

/// Price snapshot
#[derive(Debug, Clone)]
pub struct PriceSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub yes_price: f64,
    pub no_price: f64,
    pub volume: f64,
}

/// Market status
#[derive(Debug, Clone, Copy)]
pub enum MarketStatus {
    Active,
    Paused,
    Closed,
    Resolved,
}

/// WebSocket handler for real-time data
pub struct WebSocketHandler {
    pub connected: bool,
    pub subscriptions: Vec<String>,
}

impl WebSocketHandler {
    pub fn new() -> Self {
        Self {
            connected: false,
            subscriptions: Vec::new(),
        }
    }

    pub async fn connect(&mut self, _url: &str) -> Result<(), String> {
        // Simulate WebSocket connection
        self.connected = true;
        Ok(())
    }

    pub async fn subscribe(&mut self, topic: &str) -> Result<(), String> {
        self.subscriptions.push(topic.to_string());
        Ok(())
    }

    pub async fn disconnect(&mut self) {
        self.connected = false;
        self.subscriptions.clear();
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_market_manager() {
        let mut manager = MarketManager::new(1000.0, 10);
        
        manager.fetch_markets().await.unwrap();
        assert!(!manager.markets.is_empty());
        
        let markets = manager.get_liquid_markets(1000.0);
        assert!(!markets.is_empty());
    }
}
