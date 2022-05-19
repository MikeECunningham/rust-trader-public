use dec::D128;
/// Entrypoint to the application that loads the different threads for the pipeline and then hooks them together
use mimalloc::MiMalloc;
use uuid::Uuid;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[macro_use]
extern crate logging;

use std::{thread, time::{UNIX_EPOCH, SystemTime}};
use tokio::{runtime::{Builder, Runtime}, time::Instant};
use crossbeam_channel::{Sender, Receiver, unbounded};

use trader::{backend::{bybit, types::Side, binance::{types::{AccountBalanceWrapper, BinanceError}, errors::{ErrorCode, ServerNetworkErrors}}}, strategy::{types::Stage, binance::{StrategyMessage, AccountMessage}}};
use trader::strategy;
use trader::signal_handler::{bybit_handler, binance_handler};
use trader::backend::binance;
use trader::config::CONFIG;
use trader::backend::bybit::credentials::BybitCredentials;
use trader::backend::binance::credentials::BinanceCredentials;
use trader::backend::binance::types::{OrderBookSignal, Signal, DepthLimit};

/// Number of threads to have in the pool for each symbol pair added
const THREADS_PER_SYMBOL: usize = 3;

fn main() {
    if &CONFIG.env == "PRODUCTION" {
        info!("PRODUCTION ENVIRONMENT\nWE ARE LIVE");
    } else if &CONFIG.env == "TEST" {
        info!("TEST ENVIRONMENT");
    } else {
        panic!("NO ENVIRONMENT SET, CRASHING");
    }

    match &CONFIG.execution_mode {
        Some(execution_mode) => {
            if execution_mode == "PING" {
                cli_entrypoint();
            } else if execution_mode == "BYBIT" {
                automated_entrypoint_bybit();
            }
        },
        _ => { automated_entrypoint_binance() }
    }
}

fn automated_entrypoint_binance() {
    info!("Binance entrypoint go!");
    let accounts = vec![
        BinanceCredentials {
            binance_key: CONFIG.binance_key.clone(),
            binance_secret: CONFIG.binance_secret.clone(),
            binance_perpetuals_url: CONFIG.binance_perpetuals_url.clone(),
            binance_rest_url: CONFIG.binance_rest_url.clone()
        }
    ];

    let pool = Builder::new_multi_thread()
    .worker_threads(1 * THREADS_PER_SYMBOL)
    .thread_name("stream_listener_pool")
    .enable_io()
    .enable_time()
    .build()
    .expect("Failed to build async runtime for bybit order");
    for account in accounts {
        let symbol = "btcbusd".to_string();

        let (signal_tx, signal_rx): (tokio::sync::mpsc::Sender<binance::types::Signal>, tokio::sync::mpsc::Receiver<binance::types::Signal>) = tokio::sync::mpsc::channel(1);
        let (strat_tx, strat_rx): (Sender<strategy::binance::StrategyMessage>, Receiver<strategy::binance::StrategyMessage>) = unbounded();
        // Create a new signal handler, passing in channels for receiving events and updating the strategy
        let mut sig_handler = binance_handler::SignalHandler::new(strat_tx.clone(), signal_rx);
        {
            let stratshot = strat_tx.clone();
            let signal_tx = signal_tx.clone();
            let symbol = symbol.clone();
            info!("[INIT] Querying server time");
            pool.spawn( async move {
                let server_time = binance::broker::BROKER.time().await;
                let our_time: i64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().try_into().unwrap();
                binance::broker::BROKER.set_server_offset(server_time - our_time).unwrap();
                info!("[INIT] Queried server time");
                info!("[INIT] Snapshots");
                let snap = binance::market::MARKET.orderbook_snapshot(symbol, DepthLimit::Thousand).await;
                signal_tx.send(Signal::OrderBook(OrderBookSignal::OrderBookSnap(snap))).await.expect("ob snap main");
                loop {
                    match binance::broker::BROKER.account_balance().await {
                        AccountBalanceWrapper::Balance(bal) => {
                            stratshot.send(StrategyMessage::AccountMessage(AccountMessage::BalanceRefresh(bal))).unwrap();
                            break;
                        },
                        AccountBalanceWrapper::Error(_) => {},
                    }
                }
            });
        }
        {
            let signal_tx = signal_tx.clone();
            let symbol = symbol.clone();
            info!("[INIT] Spawning orderbook stream");
            pool.spawn(async move { binance::stream::orderbook::connect_orderbook(signal_tx, symbol).await; });
            info!("[INIT] Spawned orderbook stream");
        }
        {
            let signal_tx = signal_tx.clone();
            let symbol = symbol.clone();
            info!("[INIT] Spawning tradeflow stream");
            pool.spawn(async move { binance::stream::tradeflow::connect_tradeflow(signal_tx, symbol).await; });
            info!("[INIT] Spawned tradeflow stream");
        }
        {
            let signal_tx = signal_tx.clone();
            let symbol = symbol.clone();
            info!("[INIT] Spawning book ticker stream");
            pool.spawn(async move { binance::stream::book_ticker::connect_book_ticker(signal_tx, symbol).await; });
            info!("[INIT] Spawned book ticker stream");
        }
        {
            let strat_tx = strat_tx.clone();
            info!("[INIT] Spawning user data stream");
            pool.spawn(async move { binance::stream::user_data::connect_user_data(strat_tx).await; });
            info!("[INIT] Spawned user data stream");
        }

        // Spawn the main event loop threada
        thread::spawn(move || {
            info!("[INIT] Starting sig handler event loop");
            // Start the event loop
            sig_handler.event_loop();
        });
        {
            let strat_tx = strat_tx.clone();
            let symbol = symbol.clone();
            // Spawn the strategy thread
            thread::spawn(move || {
                let strategy = strategy::binance::strategy::Strategy::new(symbol, strat_tx, strat_rx);
                if let Ok(mut strategy) = strategy {
                    info!("[INIT] Starting strategy loop");
                    strategy.listen();
                    // match strategy.listen() {
                    //     Ok(_) => info!("[SHUTDOWN] Strategy exited gracefully"),
                    //     Err(reason) => {
                    //         match reason {
                    //             strategy::bybit::StrategyRuntimeError::ContactSupportError(msg) => {
                    //                 info!("[SHUTDOWN] Strategy exited with contact support error: {:?}", msg);
                    //             },
                    //         }
                    //     }
                    // }
                }
            });
        }
    }


    info!("[INIT] Initialization complete. Blocking main thread");
    // Block the main thread to prevent the program from ending
    loop{}
}

/// Called if the program is supposed to be running as an automated trader
/// application. This is the default mode for the project
fn automated_entrypoint_bybit() {

    let accounts = vec![
        BybitCredentials {
            bybit_key: CONFIG.bybit_key.clone(),
            bybit_secret: CONFIG.bybit_secret.clone(),
            bybit_perpetuals_url: CONFIG.bybit_perpetuals_url.clone(),
            bybit_perpetuals_private_url: CONFIG.bybit_perpetuals_private_url.clone(),
            bybit_rest_url: CONFIG.bybit_rest_url.clone()
        },
    ];
    // Thread pool to be utilized for spinning up multiple components of the project
    let pool = Builder::new_multi_thread()
        .worker_threads(1 * THREADS_PER_SYMBOL)
        .thread_name("stream_listener_pool")
        .enable_io()
        .enable_time()
        .build()
        .expect("Failed to build async runtime for bybit order");
    for account in accounts {

        let symbol = "BTCUSDT".to_string();

        let (signal_tx, signal_rx): (tokio::sync::mpsc::Sender<bybit::stream::Signal>, tokio::sync::mpsc::Receiver<bybit::stream::Signal>) = tokio::sync::mpsc::channel(1);
        let (strat_tx, strat_rx): (Sender<strategy::bybit::StrategyMessage>, Receiver<strategy::bybit::StrategyMessage>) = unbounded();

        // Connect each one of the stream listeners we want and have them emit their events to the event listener.
        // Each one uses a clone of the signal sender since the main one cannot be sent across to multiple async runtimes
        {
            let signal_tx = signal_tx.clone();
            let symbol = symbol.clone();
            info!("[INIT] Spawning orderbook stream");
            pool.spawn(async move { bybit::stream::orderbook::connect_orderbook(signal_tx, symbol).await; });
            info!("[INIT] Spawned orderbook stream");
        }
        {
            let signal_tx = signal_tx.clone();
            info!("[INIT] Spawning connect_trade stream");
            pool.spawn(async move { bybit::stream::trade::connect_trade(signal_tx.clone()).await; });
            info!("[INIT] Spawned connect_trade stream");
        }
        {
            let signal_tx = signal_tx.clone();
            info!("[INIT] Spawning connect_private stream");
            pool.spawn(async move { bybit::stream::private::connect_private(signal_tx.clone()).await; });
            info!("[INIT] Spawned connect_private stream");
        }

        // Create a new signal handler, passing in channels for receiving events and updating the strategy
        let mut sig_handler = bybit_handler::SignalHandler::new(strat_tx.clone(), signal_rx);
        // Wait for the initial snapshot before proceeding.
        // This will block the main thread to prevent the creation of the strategy
        // until the initial snapshot is received
        info!("[INIT] Waiting for initial snapshot");
        sig_handler.wait_for_snapshot();
        info!("[INIT] Snapshot complete");
        // Spawn the main event loop threada
        thread::spawn(move || {
            info!("[INIT] Starting sig handler event loop");
            // Start the event loop
            sig_handler.event_loop();
        });

        {
            let strat_tx = strat_tx.clone();
            let symbol = symbol.clone();
            // Spawn the strategy thread
            thread::spawn(move || {
                let strategy = strategy::bybit::strategy::Strategy::new(symbol, strat_tx, strat_rx);
                if let Ok(mut strategy) = strategy {
                    info!("[INIT] Starting strategy loop");
                    match strategy.listen() {
                        Ok(_) => info!("[SHUTDOWN] Strategy exited gracefully"),
                        Err(reason) => {
                            match reason {
                                strategy::bybit::StrategyRuntimeError::ContactSupportError(msg) => {
                                    info!("[SHUTDOWN] Strategy exited with contact support error: {:?}", msg);
                                },
                            }
                        }
                    }
                }
            });
        }
    }
    info!("[INIT] Initialization complete. Blocking main thread");
    // Block the main thread to prevent the program from ending
    loop {}
}

/// Called if the program is supposed to be running as a manual CLI application
fn cli_entrypoint() {
    // TODO implement a menu and menu logic

    info!("\nPinging {}:", CONFIG.bybit_rest_url);

    let rt = Runtime::new().expect("Failed to create async runtime");
    rt.block_on(async {
        for _ in 0..10000 {
            let timer = Instant::now();
            match bybit::broker::BROKER.ping().await {
                Ok(_) => {
                    info!("Reply: time={}us", timer.elapsed().as_micros());
                },
                Err(_) => {
                    info!("Failed Ping")
                },
            };
        }
    });
}