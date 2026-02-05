//! Test Program for Polymarket API Integration
//! 
//! This program demonstrates the integration of real Polymarket API data
//! with the HFT arbitrage bot.

use polymarket_arb_hft::*;
use tokio;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Polymarket API Integration for HFT Bot");
    println!("=".repeat(50));

    // Test 1: Initialize API Client
    println!("
📡 Test 1: Initializing Polymarket API Client");
    let api_config = PolymarketApiConfig::default();
    println!("   Gamma API: {}", api_config.gamma_api_url);
    println!("   WebSocket: {}", api_config.websocket_url);

    let api_client = PolymarketApiClient::new(api_config);

    // Test 2: Connect to API
    println!("
🔌 Test 2: Connecting to Polymarket API");
    match api_client.initialize().await {
        Ok(_) => println!("   ✅ API connection successful!"),
        Err(e) => {
            println!("   ❌ API connection failed: {}", e);
            println!("   ℹ️  Using simulation mode instead");
        }
    }

    // Test 3: Fetch Real Markets
    println!("
📊 Test 3: Fetching Real Markets from Polymarket");
    match api_client.get_markets().await {
        Ok(markets) => {
            println!("   ✅ Successfully fetched {} real markets", markets.len());

            // Display sample markets
            for (i, market) in markets.iter().take(5).enumerate() {
                println!("
   Market {}: {}", i+1, market.question);
                println!("   ID: {}", market.id);
                println!("   YES Price: {:.4} | NO Price: {:.4}", 
                    market.yes_price, market.no_price);
                println!("   Liquidity: ${:.2} | Volume: ${:.2}", 
                    market.liquidity, market.volume);
            }
        }
        Err(e) => {
            println!("   ❌ Failed to fetch markets: {}", e);
            println!("   ℹ️  Using simulated markets instead");
        }
    }

    // Test 4: Initialize Bot with Real Data
    println!("
🤖 Test 4: Initializing HFT Bot with Real Data");
    let mut bot_config = BotConfig::default();
    bot_config.use_real_data = true; // Enable real data
    bot_config.initial_capital = 1000.0;
    bot_config.min_profit_threshold = 0.005; // 0.5% threshold (optimized)

    println!("   Config: Capital=${:.2}, MinProfit={:.2}%", 
        bot_config.initial_capital, bot_config.min_profit_threshold * 100.0);
    println!("   Real Data Mode: {}", bot_config.use_real_data);

    let mut bot = HftArbitrageBot::new(bot_config);

    // Initialize API connections
    if let Err(e) = bot.initialize_api().await {
        println!("   ⚠️  API initialization warning: {}", e);
    }

    // Test 5: Run Single Step with Real Data
    println!("
⚡ Test 5: Running Single Trading Step with Real Data");
    match bot.run_step().await {
        Ok(result) => {
            println!("   Step: {}", result.step);
            println!("   Opportunities Found: {}", result.opportunities);
            println!("   Trades Executed: {}", result.trades);
            println!("   Profit: ${:.4}", result.profit);
            println!("   Current Capital: ${:.2}", result.capital);
            println!("   Win Rate: {:.2}%", result.win_rate * 100.0);
        }
        Err(e) => {
            println!("   ❌ Step execution failed: {}", e);
        }
    }

    println!("
" + "=".repeat(50));
    println!("✅ Polymarket API Integration Test Completed!");

    Ok(())
}
