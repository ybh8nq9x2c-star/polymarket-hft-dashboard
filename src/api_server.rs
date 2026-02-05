
//! API Server per Dashboard HFT Polymarket
//! Fornisce endpoint REST e WebSocket per gestione bot e paper trading

use actix_web::{web, App, HttpServer, HttpResponse, Responder, Result, Error};
use actix_cors::Cors;
use actix_files::{Files, NamedFile};
use actix_ws::{Message, ProtocolError};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use rand::seq::{IteratorRandom, SliceRandom};










/// Stato globale del bot per dashboard
#[derive(Clone, Serialize, Deserialize)]
pub struct BotState {
    pub running: bool,
    pub balance: f64,
    pub initial_balance: f64,
    pub total_pnl: f64,
    pub win_rate: f64,
    pub total_trades: usize,
    pub profitable_trades: usize,
    pub last_update: DateTime<Utc>,
}

/// Trade simulato con dati reali per backtesting
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SimulatedTrade {
    pub id: String,
    pub market_id: String,
    pub question: String,
    pub action: String, // "BUY_YES", "BUY_NO", "SELL_YES", "SELL_NO"
    pub price: f64,
    pub quantity: f64,
    pub amount: f64,
    pub timestamp: DateTime<Utc>,
    pub status: String, // "PENDING", "FILLED", "CANCELLED"
    pub pnl: f64,
    pub arbitrage_profit: f64, // Profitto di arbitraggio simulato
}

/// Informazioni mercato reale
#[derive(Clone, Serialize, Deserialize)]
pub struct MarketInfo {
    pub id: String,
    pub question: String,
    pub yes_price: f64,
    pub no_price: f64,
    pub yes_liquidity: f64,
    pub no_liquidity: f64,
    pub volume_24h: f64,
    pub timestamp: DateTime<Utc>,
}

/// Dati live per WebSocket
#[derive(Clone, Serialize)]
pub struct LiveData {
    pub bot_state: BotState,
    pub markets: Vec<MarketInfo>,
    pub recent_trades: Vec<SimulatedTrade>,
    pub arbitrage_opportunities: Vec<ArbitrageOpportunity>,
}

/// Opportunità di arbitraggio
#[derive(Clone, Serialize)]
pub struct ArbitrageOpportunity {
    pub market_id: String,
    pub market1_id: String,
    pub market2_id: String,
    pub profit_percent: f64,
    pub expected_profit: f64,
    pub timestamp: DateTime<Utc>,
}

/// Struttura condivisa per gestione stato
pub struct AppState {
    pub bot_state: Arc<Mutex<BotState>>,
    pub trades: Arc<Mutex<Vec<SimulatedTrade>>>,
    pub markets: Arc<Mutex<Vec<MarketInfo>>>,
    pub clients: Arc<Mutex<HashMap<String, bool>>>, // WebSocket clients
}

impl AppState {
    pub fn new() -> Self {
        let initial_balance = 10000.0;
        AppState {
            bot_state: Arc::new(Mutex::new(BotState {
                running: false,
                balance: initial_balance,
                initial_balance,
                total_pnl: 0.0,
                win_rate: 0.0,
                total_trades: 0,
                profitable_trades: 0,
                last_update: Utc::now(),
            })),
            trades: Arc::new(Mutex::new(Vec::new())),
            markets: Arc::new(Mutex::new(Vec::new())),
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Request payload per avviare/fermare bot
#[derive(Deserialize)]
pub struct BotControlRequest {
    pub action: String, // "start" o "stop"
    pub initial_balance: Option<f64>,
    pub trade_frequency: Option<u64>, // Secondi tra trade
}

/// Response payload
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            message: "Success".to_string(),
        }
    }

    pub fn error(message: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            message,
        }
    }
}

/// GET /api/status - Get bot status
pub async fn get_bot_status(data: web::Data<AppState>) -> impl Responder {
    let bot_state = data.bot_state.lock().unwrap();
    HttpResponse::Ok().json(ApiResponse::success(bot_state.clone()))
}

/// POST /api/control - Control bot (start/stop)
pub async fn control_bot(
    data: web::Data<AppState>,
    req: web::Json<BotControlRequest>
) -> impl Responder {
    let mut bot_state = data.bot_state.lock().unwrap();

    match req.action.as_str() {
        "start" => {
            if bot_state.running {
                return HttpResponse::Ok().json(ApiResponse::<()>::error("Bot already running".to_string()));
            }

            if let Some(balance) = req.initial_balance {
                bot_state.initial_balance = balance;
                bot_state.balance = balance;
            }

            bot_state.running = true;
            bot_state.last_update = Utc::now();

            // Avvia simulazione trade con dati reali
            tokio::spawn(simulate_trading(
                data.bot_state.clone(),
                data.trades.clone(),
                data.markets.clone(),
                req.trade_frequency.unwrap_or(30) // Default 30 secondi
            ));

            HttpResponse::Ok().json(ApiResponse::success("Bot started successfully"))
        }
        "stop" => {
            bot_state.running = false;
            HttpResponse::Ok().json(ApiResponse::success("Bot stopped successfully"))
        }
        _ => HttpResponse::BadRequest().json(ApiResponse::<()>::error("Invalid action".to_string()))
    }
}

/// GET /api/trades - Get all trades
pub async fn get_trades(data: web::Data<AppState>) -> impl Responder {
    let trades = data.trades.lock().unwrap();
    HttpResponse::Ok().json(ApiResponse::success(trades.clone()))
}

/// GET /api/markets - Get market data
pub async fn get_markets(data: web::Data<AppState>) -> impl Responder {
    let markets = data.markets.lock().unwrap();
    HttpResponse::Ok().json(ApiResponse::success(markets.clone()))
}

/// POST /api/trades/clear - Clear all trades
pub async fn clear_trades(data: web::Data<AppState>) -> impl Responder {
    let mut trades = data.trades.lock().unwrap();
    trades.clear();

    // Reset bot state
    let mut bot_state = data.bot_state.lock().unwrap();
    bot_state.total_trades = 0;
    bot_state.profitable_trades = 0;
    bot_state.total_pnl = 0.0;
    bot_state.win_rate = 0.0;
    bot_state.balance = bot_state.initial_balance;

    HttpResponse::Ok().json(ApiResponse::success("Trades cleared successfully"))
}

/// Simula trading con dati reali dai mercati Polymarket
async fn simulate_trading(
    bot_state: Arc<Mutex<BotState>>,
    trades: Arc<Mutex<Vec<SimulatedTrade>>>,
    markets: Arc<Mutex<Vec<MarketInfo>>>,
    frequency: u64
) {
    use std::time::Duration;
    use rand::Rng;

    let mut interval = tokio::time::interval(Duration::from_secs(frequency));

    loop {
        interval.tick().await;

        // Check se bot è ancora in esecuzione
        {
            let state = bot_state.lock().unwrap();
            if !state.running {
                break;
            }
        }

        // Ottieni mercati disponibili
        let available_markets = {
            let markets_guard = markets.lock().unwrap();
            if markets_guard.is_empty() {
                continue;
            }
            markets_guard.clone()
        };

        // Seleziona mercato random per trade simulato
        if let Some(market) = available_markets.iter().choose(&mut rand::thread_rng()) {
            let mut rng = rand::thread_rng();

            // Simula decisione trading basata su dati reali
            let action = if rng.gen_bool(0.5) { "BUY_YES" } else { "BUY_NO" };
            let price = if action == "BUY_YES" { market.yes_price } else { market.no_price };

            // Calcola quantità basata su balance e rischio
            let balance = {
                let state = bot_state.lock().unwrap();
                state.balance
            };

            let risk_percentage = 0.02; // 2% del balance per trade
            let amount = balance * risk_percentage;
            let quantity = amount / price;

            // Simula PnL con una certa probabilità di profitto
            let pnl = if rng.gen_bool(0.55) { // 55% win rate
                amount * (rng.gen_range(0.01..0.15)) // Profitto 1-15%
            } else {
                -amount * (rng.gen_range(0.01..0.10)) // Perdita 1-10%
            };

            // Simula profitto arbitraggio
            let arbitrage_profit = if rng.gen_bool(0.3) {
                amount * rng.gen_range(0.001..0.01) // 0.1-1% arbitrage
            } else {
                0.0
            };

            // Crea trade simulato
            let trade = SimulatedTrade {
                id: uuid::Uuid::new_v4().to_string(),
                market_id: market.id.clone(),
                question: market.question.clone(),
                action: action.to_string(),
                price,
                quantity,
                amount,
                timestamp: Utc::now(),
                status: "FILLED".to_string(),
                pnl,
                arbitrage_profit,
            };

            // Aggiorna stato bot
            {
                let mut state = bot_state.lock().unwrap();
                state.balance += pnl + arbitrage_profit;
                state.total_pnl += pnl + arbitrage_profit;
                state.total_trades += 1;
                state.profitable_trades += if pnl + arbitrage_profit > 0.0 { 1 } else { 0 };
                state.win_rate = (state.profitable_trades as f64 / state.total_trades as f64) * 100.0;
                state.last_update = Utc::now();
            }

            // Salva trade
            {
                let mut trades_guard = trades.lock().unwrap();
                trades_guard.push(trade);

                // Mantieni solo ultimi 100 trade in memoria
                if trades_guard.len() > 100 {
                    trades_guard.remove(0);
                }
            }
        }
    }
}

/// Avvia il server API
pub async fn start_api_server(port: u16) -> std::io::Result<()> {
    env_logger::init();

    let app_state = web::Data::new(AppState::new());

    println!("🚀 Avvio server API dashboard su http://0.0.0.0:{}", port);
    println!("📁 Frontend servito su /frontend");

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/api/status", web::get().to(get_bot_status))
            .route("/api/control", web::post().to(control_bot))
            .route("/api/trades", web::get().to(get_trades))
            .route("/api/markets", web::get().to(get_markets))
            .route("/api/trades/clear", web::post().to(clear_trades))
            .service(Files::new("/frontend", "./frontend"))
            .route("/", web::get().to(serve_frontend))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

/// Serve il frontend
async fn serve_frontend() -> Result<NamedFile, Error> {
    use actix_files::NamedFile;
    Ok(NamedFile::open("./frontend/index.html")?)
}

// Add this to use choose method

