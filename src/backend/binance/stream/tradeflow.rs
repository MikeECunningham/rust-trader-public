use std::time::{Instant, Duration};

use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use futures::{SinkExt, StreamExt};
use reqwest::StatusCode;
use tokio::task;
use tokio::sync::mpsc::{Sender, Receiver, channel};
use tokio::time;

use crate::backend::binance::types::{WebsocketSubscribe, Signal, TradeFlows, FuturesTrades, StreamWrapper};
use crate::config::CONFIG;

pub async fn connect_tradeflow(sender: Sender<Signal>, symbol: String) {
    let trade_arg = format!("{}@aggTrade", symbol);
    // let adl_arg = format!("{}@forceOrder", symbol);
    // let url = format!("{}/stream?streams={}/{}", CONFIG.binance_perpetuals_url.clone(), trade_arg, adl_arg);
    let url = format!("{}/ws/{}", CONFIG.binance_perpetuals_url.clone(), trade_arg);
    let (ws_stream, res) = connect_async(url)
        .await
        .expect("error building stream");
    if !res.status().is_informational() && !res.status().is_success() { panic!("panic stream: {:?}\nres: {:?}", ws_stream, res); }

    let (mut write, mut read) = ws_stream.split();
    // let obsub = serde_json::to_string(&WebsocketSubscribe {
    //     method: "SUBSCRIBE".to_string(),
    //     params: vec![args],
    //     id: 1,
    // }).expect("ob error stringifying query json");
    // write.send(Message::Text(obsub)).await.expect("ob init error");

    loop {
        match read.next().await
        .expect("TR stream error getting message")
        .expect("TR stream error getting message") {
            Message::Text(txt) => {
                let timer = Instant::now();
                let mut tr = serde_json::from_str::<FuturesTrades>(&txt.to_string()).expect("Deser TR went wrong");
                tr.test_timer = timer;
                sender.send(Signal::TradeFlows(TradeFlows::Trades(tr))).await.expect("err sending ob out of ws");
            },
            Message::Binary(_) => todo!(),
            Message::Ping(_) => {
                write.send(Message::Pong(vec![]))
                    .await.expect("TRFL error attempting pong");
            },
            Message::Pong(_) => info!("Pong received for some reason"),
            Message::Close(a) => {
                let frm = a.expect("something went wrong getting the reason info");
                println!(
                    "The server shut down the TR stream because of {}, code: {}",
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