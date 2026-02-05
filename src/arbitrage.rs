//! Arbitrage detection module
//!
//! Implements:
//! 1. YES/NO arbitrage: YES_price + NO_price < 1
//! 2. Graph-based arbitrage detection
//! 3. Modified Moore-Bellman-Ford (MMBF) algorithm

use crate::types::*;
use fxhash::FxHashMap;
use std::collections::HashSet;

/// Arbitrage detector for YES/NO arbitrage
pub struct ArbitrageDetector {
    pub min_profit: f64,
    pub min_liquidity: f64,
}

impl ArbitrageDetector {
    pub fn new(min_profit: f64, min_liquidity: f64) -> Self {
        Self { 
            min_profit: 0.005,  // Ridotto da 1% a 0.5% per aumentare frequenza trade
            min_liquidity 
        }
    }

    /// Detect YES/NO arbitrage opportunity
    pub fn detect_yes_no_arbitrage(&self, market: &MarketData) -> Option<ArbitrageOpportunity> {
        let sum = market.yes_price + market.no_price;
        
        // Arbitrage condition: YES + NO < 1
        if sum >= 1.0 { 
            return None; 
        }

        let arb_profit = 1.0 - sum;
        
        // Check minimum profit threshold
        if arb_profit < self.min_profit { 
            return None; 
        }

        // Check liquidity
        let total_liquidity = market.yes_liquidity + market.no_liquidity;
        if total_liquidity < self.min_liquidity { 
            return None; 
        }

        // Calculate confidence score
        let liquidity_score = (total_liquidity / 10000.0).min(1.0);
        let profit_score = (arb_profit / 0.05).min(1.0);
        let volume_score = (market.volume_24h / 50000.0).min(1.0);
        let confidence = liquidity_score * 0.3 + profit_score * 0.5 + volume_score * 0.2;

        Some(ArbitrageOpportunity {
            market_id: market.id.clone(),
            question: market.question.clone(),
            arb_type: ArbType::YesNoSimple,
            profit: arb_profit,
            roi_pct: arb_profit * 100.0,
            confidence,
            yes_price: market.yes_price,
            no_price: market.no_price,
            sum_price: sum,
            liquidity: total_liquidity,
            timestamp: market.timestamp,
            legs: Some(vec![
                ArbitrageLeg {
                    market_id: market.id.clone(),
                    token_type: TokenType::Yes,
                    direction: Direction::Buy,
                    price: market.yes_price,
                    quantity: 0.0,
                },
                ArbitrageLeg {
                    market_id: market.id.clone(),
                    token_type: TokenType::No,
                    direction: Direction::Buy,
                    price: market.no_price,
                    quantity: 0.0,
                },
            ]),
            path: None,
        })
    }

    /// Scan all markets for arbitrage opportunities
    pub fn scan_markets(&self, markets: &[MarketData]) -> Vec<ArbitrageOpportunity> {
        markets.iter()
            .filter_map(|market| self.detect_yes_no_arbitrage(market))
            .collect()
    }
}

/// Graph-based arbitrage detector using Modified Moore-Bellman-Ford
pub struct GraphArbitrageDetector {
    pub markets: FxHashMap<String, MarketData>,
}

impl GraphArbitrageDetector {
    pub fn new() -> Self {
        Self { markets: FxHashMap::default() }
    }

    pub fn add_market(&mut self, market: MarketData) {
        self.markets.insert(market.id.clone(), market);
    }

    /// Detect arbitrage cycles using MMBF algorithm
    pub fn detect_arbitrage_cycles(&self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        let graph = self._build_price_graph();
        let cycles = self._mmbf_algorithm(&graph);
        
        for cycle in cycles {
            if let Some(opp) = self._cycle_to_opportunity(&cycle) {
                opportunities.push(opp);
            }
        }
        opportunities
    }

    /// Build price graph for arbitrage detection
    fn _build_price_graph(&self) -> FxHashMap<String, FxHashMap<String, f64>> {
        let mut graph: FxHashMap<String, FxHashMap<String, f64>> = FxHashMap::default();
        
        for (market_id, market) in &self.markets {
            // Use negative log prices for shortest path conversion
            let yes_weight = -market.yes_price.ln();
            let no_weight = -market.no_price.ln();
            
            // Create bidirectional edges
            graph.entry(format!("{}-YES", market_id))
                .or_insert_with(FxHashMap::default)
                .insert(format!("{}-NO", market_id), yes_weight);
            
            graph.entry(format!("{}-NO", market_id))
                .or_insert_with(FxHashMap::default)
                .insert(format!("{}-YES", market_id), no_weight);
        }
        graph
    }

    /// Modified Moore-Bellman-Ford algorithm for cycle detection
    fn _mmbf_algorithm(&self, graph: &FxHashMap<String, FxHashMap<String, f64>>) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut dist: FxHashMap<String, f64> = FxHashMap::default();
        let mut pred: FxHashMap<String, Option<String>> = FxHashMap::default();
        
        // Initialize distances
        for node in graph.keys() {
            dist.insert(node.clone(), f64::MAX);
            pred.insert(node.clone(), None);
        }
        
        // Run MMBF from each node
        for start in graph.keys() {
            dist.insert(start.clone(), 0.0);
            
            // Relax edges V-1 times
            for _ in 0..graph.len() {
                for (u, neighbors) in graph.iter() {
                    for (v, weight) in neighbors.iter() {
                        let du = *dist.get(u).unwrap_or(&f64::MAX);
                        let dv = *dist.get(v).unwrap_or(&f64::MAX);
                        
                        if du + weight < dv {
                            dist.insert(v.clone(), du + weight);
                            pred.insert(v.clone(), Some(u.clone()));
                        }
                    }
                }
            }
            
            // Check for negative cycles (arbitrage)
            for (u, neighbors) in graph.iter() {
                for (v, weight) in neighbors.iter() {
                    let du = *dist.get(u).unwrap_or(&f64::MAX);
                    let dv = *dist.get(v).unwrap_or(&f64::MAX);
                    
                    if du + weight < dv {
                        // Found negative cycle
                        if let Some(cycle) = self._extract_cycle(&pred, v) {
                            cycles.push(cycle);
                        }
                    }
                }
            }
            
            // Reset for next iteration
            for node in graph.keys() {
                dist.insert(node.clone(), f64::MAX);
                pred.insert(node.clone(), None);
            }
        }
        cycles
    }

    /// Extract arbitrage cycle from predecessor map
    fn _extract_cycle(&self, pred: &FxHashMap<String, Option<String>>, start: &str) -> Option<Vec<String>> {
        let mut cycle = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut current = Some(start.to_string());
        
        while let Some(curr) = current {
            if visited.contains(&curr) {
                let idx = cycle.iter().position(|x| x == &curr)?;
                return Some(cycle[idx..].to_vec());
            }
            
            visited.insert(curr.clone());
            cycle.push(curr.clone());
            current = pred.get(&curr)?.clone();
        }
        None
    }

    /// Convert detected cycle to arbitrage opportunity
    fn _cycle_to_opportunity(&self, cycle: &[String]) -> Option<ArbitrageOpportunity> {
        if cycle.len() < 2 { return None; }

        let mut profit = 1.0;
        for node in cycle {
            if let Some((market_id, token_type)) = self._parse_node(node) {
                if let Some(market) = self.markets.get(&market_id) {
                    let price = match token_type {
                        TokenType::Yes => market.yes_price,
                        TokenType::No => market.no_price,
                    };
                    profit *= price;
                }
            }
        }
        
        let arb_profit = 1.0 - profit;
        if arb_profit <= 0.001 { return None; }  // Minimum 0.1% profit

        Some(ArbitrageOpportunity {
            market_id: cycle.first().unwrap().clone(),
            question: "Graph arbitrage".to_string(),
            arb_type: ArbType::GraphArbitrage,
            profit: arb_profit,
            roi_pct: arb_profit * 100.0,
            confidence: 0.7,
            yes_price: 0.0,
            no_price: 0.0,
            sum_price: profit,
            liquidity: 0.0,
            timestamp: chrono::Utc::now(),
            legs: None,
            path: Some(cycle.to_vec()),
        })
    }

    /// Parse node identifier into market_id and token type
    fn _parse_node(&self, node: &str) -> Option<(String, TokenType)> {
        let parts: Vec<&str> = node.rsplitn(2, '-').collect();
        if parts.len() != 2 { return None; }
        
        let token_type = match parts[0] {
            "YES" => TokenType::Yes,
            "NO" => TokenType::No,
            _ => return None,
        };
        Some((parts[1].to_string(), token_type))
    }
}
