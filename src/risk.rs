//! Risk management module
//!
//! Implements:
//! 1. Value at Risk (VaR) calculation
//! 2. Sharpe Ratio calculation
//! 3. Maximum Drawdown tracking
//! 4. Risk controls and limits

use crate::types::*;
use fxhash::FxHashMap;

/// Risk manager
pub struct RiskManager {
    pub metrics: RiskMetrics,
    pub trade_history: Vec<f64>,
    pub consecutive_losses: u32,
    pub daily_loss: f64,
    pub peak_capital: f64,
    pub low_capital: f64,
}

impl RiskManager {
    pub fn new(
        daily_loss_limit: f64,
            max_consecutive_losses: u32,
            max_drawdown: f64,
            max_position_size: f64,   // Parametro senza default
            max_daily_loss_pct: f64,   // Parametro senza default
            max_consecutive_losses_limit: u32,  // Parametro senza default
        ) -> Self {
        Self {
            metrics: RiskMetrics {
                var_95: 0.0,
                daily_loss_limit,
                max_consecutive_losses,
                max_drawdown,
                current_drawdown: 0.0,
                sharpe_ratio: 0.0,
            },
            trade_history: Vec::new(),
            consecutive_losses: 0,
            daily_loss: 0.0,
            peak_capital: 0.0,
            low_capital: 0.0,
        }
    }

    /// Update risk metrics after a trade
    pub fn update(&mut self, profit: f64, capital: f64) {
        self.trade_history.push(profit);
        
        if self.trade_history.len() > 1000 {
            self.trade_history.remove(0);
        }
        
        if profit < 0.0 {
            self.consecutive_losses += 1;
            self.daily_loss += profit.abs();
        } else {
            self.consecutive_losses = 0;
        }
        
        if capital > self.peak_capital {
            self.peak_capital = capital;
            self.low_capital = capital;
        }
        
        if capital < self.low_capital {
            self.low_capital = capital;
        }
        
        self.metrics.current_drawdown = (self.peak_capital - capital) / self.peak_capital;
        self.metrics.var_95 = self.calculate_var_95();
        self.metrics.sharpe_ratio = self.calculate_sharpe_ratio();
    }

    /// Calculate 95% Value at Risk
    pub fn calculate_var_95(&self) -> f64 {
        if self.trade_history.len() < 20 {
            return 0.0;
        }
        
        let mut losses: Vec<f64> = self.trade_history.iter().filter(|&&p| p < 0.0).cloned().collect();
        losses.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = (losses.len() as f64 * 0.95).floor() as usize;
        if index < losses.len() {
            -losses[index]
        } else {
            0.0
        }
    }

    /// Calculate Sharpe Ratio
    pub fn calculate_sharpe_ratio(&self) -> f64 {
        if self.trade_history.len() < 10 {
            return 0.0;
        }
        
        let mean = self.trade_history.iter().sum::<f64>() / self.trade_history.len() as f64;
        let variance = self.trade_history.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / self.trade_history.len() as f64;
        let std = variance.sqrt();
        
        if std < 1e-9 {
            return 0.0;
        }
        
        mean / std * (252.0_f64).sqrt()  // Annualized
    }

    /// Check if trade should be allowed
    pub fn can_trade(&self, _capital: f64) -> bool {
        if self.daily_loss >= self.metrics.daily_loss_limit {
            return false;
        }
        
        if self.consecutive_losses >= self.metrics.max_consecutive_losses {
            return false;
        }
        
        if self.metrics.current_drawdown >= self.metrics.max_drawdown {
            return false;
        }
        
        true
    }

    /// Get current risk status
    pub fn get_risk_status(&self) -> RiskStatus {
        RiskStatus {
            can_trade: self.can_trade(self.peak_capital - self.metrics.current_drawdown * self.peak_capital),
            consecutive_losses: self.consecutive_losses,
            daily_loss_pct: (self.daily_loss / self.peak_capital) * 100.0,
            current_drawdown_pct: self.metrics.current_drawdown * 100.0,
            var_95: self.metrics.var_95,
            sharpe_ratio: self.metrics.sharpe_ratio,
        }
    }

    /// Reset daily limits
    pub fn reset_daily(&mut self) {
        self.daily_loss = 0.0;
    }
}

/// Risk status
#[derive(Debug, Clone)]
pub struct RiskStatus {
    pub can_trade: bool,
    pub consecutive_losses: u32,
    pub daily_loss_pct: f64,
    pub current_drawdown_pct: f64,
    pub var_95: f64,
    pub sharpe_ratio: f64,
}

/// Position sizer using Kelly Criterion
pub struct PositionSizer {
    pub kelly_fraction: f64,
    pub max_position_pct: f64,
    pub min_position: f64,
}

impl PositionSizer {
    pub fn new(kelly_fraction: f64, max_position_pct: f64, min_position: f64) -> Self {
        Self {
            kelly_fraction,
            max_position_pct,
            min_position,
        }
    }

    /// Calculate optimal position size using modified Kelly
    pub fn calculate_position(
        &self,
        capital: f64,
        win_rate: f64,
        avg_win: f64,
        avg_loss: f64,
        confidence: f64,
    ) -> f64 {
        if win_rate <= 0.0 || avg_loss <= 0.0 {
            return self.min_position;
        }
        
        // Modified Kelly criterion
        let win_loss_ratio = avg_win / avg_loss;
        let kelly_pct = (win_rate * win_loss_ratio - (1.0 - win_rate)) / win_loss_ratio;
        
        // Apply Kelly fraction and confidence
        let adjusted_kelly = kelly_pct * self.kelly_fraction * confidence;
        
        // Cap at maximum position
        let position = capital * adjusted_kelly.min(self.max_position_pct);
        
        position.max(self.min_position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_manager() {
        let mut rm = RiskManager::new(50.0, 5, 0.15);
        
        for i in 0..10 {
            let profit = if i % 2 == 0 { 5.0 } else { -2.0 };
            rm.update(profit, 1000.0 + i as f64 * 3.0);
        }
        
        assert!(rm.trade_history.len() == 10);
        assert!(rm.calculate_sharpe_ratio() > 0.0);
    }
}
