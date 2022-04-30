use std::time::{Instant, Duration};

use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use futures::{SinkExt, StreamExt};
use reqwest::StatusCode;
use tokio::task;
use tokio::sync::mpsc::{Sender, Receiver, channel};
use tokio::time;

use crate::backend::binance::types::{WebsocketSubscribe, Orders, OrderbookResponse, OrderBookSignal, Signal, BestLevel};
use crate::config::CONFIG;

pub async fn connect_book_ticker(sender: Sender<Signal>, symbol: String) {
    let args = format!("{}@bookTicker", symbol);
    let url = format!("{}/ws/{}", CONFIG.binance_perpetuals_url.clone(), args);
    let (ws_stream, res) = connect_async(url)
        .await
        .expect("error building stream");
    if !res.status().is_informational() && !res.status().is_success() { panic!("panic stream: {:?}\nres: {:?}", ws_stream, res); }
    let (mut write, mut read) = ws_stream.split();

    loop {
        match read.next().await.expect("BT stream error getting message").expect("BT stream error getting message") {
            Message::Text(txt) => {
                let timer = Instant::now();
                let mut bt = serde_json::from_str::<BestLevel>(&txt.to_string()).expect("Deser BT went wrong");
                bt.test_timer = timer;
                sender.send(Signal::OrderBook(OrderBookSignal::BestLevels(bt))).await.expect("err sending bt out of ws");
            },
            Message::Binary(_) => todo!(),
            Message::Ping(_) => {
                write.send(Message::Pong(vec![]))
                    .await.expect("BT error attempting pong");
            },
            Message::Pong(_) => info!("Pong received for some reason"),
            Message::Close(a) => {
                let frm = a.expect("something went wrong getting the reason info");
                println!(
                    "The server shut down the BT stream because of {}, code: {}",
                    frm.reason, frm.code
                );
                write
                    .close()
                    .await
                    .expect("somehow closing the stream messed up");
            },
            Message::Frame(_) => todo!(),
        }
    }
}