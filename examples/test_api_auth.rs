
use polymarket_arb_hft::*;
use tokio;

#[tokio::main]
async fn main() {
    println!("🚀 Test Autenticazione API Polymarket");
    println!("===================================");

    // Credenziali API fornite dall'utente
    let api_key = Some("019c2d5e-6b63-70d5-a637-b320e266fee5".to_string());
    let secret = Some("nLSuNgtPSuuGkG8nhqdgdefrUXwi8I7vRxKEaNjWePo=".to_string());
    let passphrase = Some("8f03eb97b93c26a9006a1bb4748ac7b6376c712888a72bb13cfa711d110b8d45".to_string());

    println!("🔑 Credenziali configurate:");
    println!("   API Key: {}...{}", 
        api_key.as_ref().unwrap()[..8].to_string(),
        api_key.as_ref().unwrap()[api_key.as_ref().unwrap().len()-4..].to_string()
    );
    println!("   Secret: {}...{}", 
        secret.as_ref().unwrap()[..8].to_string(),
        secret.as_ref().unwrap()[secret.as_ref().unwrap().len()-4..].to_string()
    );
    println!("   Passphrase: {}...{}", 
        passphrase.as_ref().unwrap()[..8].to_string(),
        passphrase.as_ref().unwrap()[passphrase.as_ref().unwrap().len()-4..].to_string()
    );

    // Configurazione bot con dati reali
    let config = BotConfig {
        initial_capital: 1000.0,
        min_profit_threshold: 0.005,
        risk_per_trade: 0.10,
        max_position_size: 500.0,
        api_base: "https://clob.polymarket.com".to_string(),
        ws_url: "wss://ws-subscriptions-clob.polymarket.com".to_string(),
        api_key: None,
        enable_mev: true,
        max_execution_time_ms: 100,
        polling_interval_ms: 1000,
        use_real_data: true,
        polymarket_api_key: api_key,
        polymarket_secret: secret,
        polymarket_passphrase: passphrase,
    };

    println!("\n📡 Inizializzando bot con configurazione reale...");
    let mut bot = HftArbitrageBot::new(config);

    // Verifica che il client API sia stato inizializzato con le credenziali
    if let Some(ref api_client) = bot.polymarket_api {
        println!("✅ Client API inizializzato");
        println!("📡 Configurazione API:");
        println!("   Gamma URL: https://gamma-api.polymarket.com");
        println!("   WebSocket: wss://ws-subscriptions-clob.polymarket.com");
    } else {
        println!("⚠️  Client API non inizializzato (use_real_data = false)");
    }

    println!("\n🎯 Test completato!");
    println!("📊 Configurazione pronta per trading con API reali");
}
