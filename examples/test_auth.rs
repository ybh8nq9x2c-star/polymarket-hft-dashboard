// Test autenticazione con credenziali API reali
use polymarket_arb_hft::{PolymarketApiClient, PolymarketApiConfig, MarketData};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Test autenticazione API Polymarket");
    println!("==================================================");

    // Configurazione con credenziali API reali
    let config = PolymarketApiConfig {
        gamma_api_url: "https://gamma-api.polymarket.com".to_string(),
        clob_api_url: "https://clob.polymarket.com".to_string(),
        websocket_url: "wss://ws-subscriptions-clob.polymarket.com".to_string(),
        api_key: Some("019c2d5e-6b63-70d5-a637-b320e266fee5".to_string()),
    };

    println!("✅ Configurazione completata con credenziali API");
    println!("   API Key: {}", "019c2d5e...fee5");

    // Crea il client API con tutti i 4 parametri richiesti
    let api_client = PolymarketApiClient::new(
        config,
        Some("019c2d5e-6b63-70d5-a637-b320e266fee5".to_string()),
        Some("nLSuNgtPSuuGkG8nhqdgdefrUXwi8I7vRxKEaNjWePo=".to_string()),
        Some("8f03eb97b93c26a9006a1bb4748ac7b6376c712888a72bb13cfa711d110b8d45".to_string())
    );
    println!("✅ Client API creato");

    // Test autenticazione recuperando mercati
    println!("
🔄 Test autenticazione recupero mercati...");
    let markets: Vec<MarketData> = api_client.get_markets().await?;

    println!("✅ Autenticazione RIUSCITA!");
    println!("   Recuperati {} mercati", markets.len());

    if !markets.is_empty() {
        println!("
📊 Esempio mercato:");
        let market = &markets[0];
        println!("   ID: {}", market.id);
        println!("   Domanda: {}", market.question);
        println!("   Prezzo YES: {:.4}", market.yes_price);
        println!("   Prezzo NO: {:.4}", market.no_price);
        println!("   Liquidità YES: {:.2}", market.yes_liquidity);
        println!("   Volume 24h: {:.2}", market.volume_24h);
    }

    println!("
==================================================");
    println!("🎉 Test completato con successo!");
    Ok(())
}
