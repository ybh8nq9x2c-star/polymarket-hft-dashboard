
//! Dashboard HFT Polymarket - Main Entry Point
//! Avvia il server API e la dashboard professionale

use polymarket_arb_hft::api_server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Avvio Dashboard HFT Polymarket");
    println!("{}", String::from("=").repeat(50));
    println!("📡 API Server: http://0.0.0.0:8080");
    println!("🌐 Dashboard: http://localhost:8080");
    println!("📊 Features:");
    println!("   - Paper Trading con dati reali");
    println!("   - Backtesting avanzato");
    println!("   - Statistiche in tempo reale");
    println!("   - Trade simulati con dati Polymarket");
    println!("{}", String::from("=").repeat(50));
    println!("⚡ Pronto! Apri http://localhost:8080 nel browser
");

    api_server::start_api_server(8080).await
}
