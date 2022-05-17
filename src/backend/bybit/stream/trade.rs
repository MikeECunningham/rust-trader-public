use tokio::sync::mpsc::{Sender, Receiver, channel};
use std::{thread, time::Duration};
use tokio::runtime::Runtime;

use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use futures::{SinkExt, StreamExt};

use crate::backend::bybit::stream::{BybitStream, Signal, Signal::{Tradeflow}, TradeTicks};
use crate::backend::bybit::stream::ArgType;
use crate::backend::bybit::stream::WebsocketMessager;
use crate::backend::bybit::stream::WebsocketPing;
use crate::backend::bybit::stream::WebsocketSubscribe;
use crate::config::CONFIG;
use tokio::time;

pub async fn connect_trade(sender: Sender<Signal>) {
    let stream =  BybitStream::new();
    info!("[INIT] Bybit trade socket connecting...");
    let (ws_stream, _res) = connect_async(CONFIG.bybit_perpetuals_url.clone())
        .await
        .expect("error building stream");
    info!("[INIT] Connect trade async done");
    let (mut write, mut read) = ws_stream.split();
    let obsub = serde_json::to_string(&WebsocketSubscribe {
        message_type: "subscribe".to_string(),
        args: vec![ArgType::String("trade.BTCUSDT".to_string())],
    })
    .expect("something went wrong stringifying query json");
    write
        .send(Message::Text(obsub))
        .await
        .expect("initialization of the orderbook went wrong");

    let (send, mut rec): (Sender<WebsocketMessager>, Receiver<WebsocketMessager>) = channel(1);
    let ping_send = send.clone();
    let msg_send = send.clone();

    let rt = Runtime::new().expect("Failed to create runtime");
    rt.spawn(async move {
        let mut interval = time::interval(Duration::from_millis(25000));
        loop {
            interval.tick().await;
            if ping_send.send(WebsocketMessager::Ping()).await.is_err() {
                panic!("something went wrong sending private ping to main ws thread");
            }
        }
    });
    rt.spawn(async move {
        loop {
            let msg = match read
                .next()
                .await
                .expect("something went wrong waiting for next message") {
                    Ok(message) => message,
                    Err(err) => panic!("Something went wrong getting next message: {}", err),
                };
                // .expect("something went wrong getting next message");
            if msg_send.send(WebsocketMessager::Message(msg)).await.is_err() {
                panic!("something went wrong sending private msg to main ws thread");
            }
        }
    });

    loop {
        match rec.recv().await {
            Some(m) => match m {
                WebsocketMessager::Message(msg) => {
                    match msg {
                        Message::Text(txt) => {
                            // println!("text: {}", txt);

                            let trade_txt =
                                serde_json::from_str::<TradeTicks>(&txt.to_string())
                                    .expect("Deserializing a tick went wrong");
                            match trade_txt {
                                TradeTicks::TradeTick(trade) => {
                                    // println!("parsed: {}, {}", parsed_txt.data[0].side, parsed_txt.data[0].size);
                                    if stream.trades_activated {
                                        sender
                                            .send(Tradeflow(trade)).await
                                            .expect("problem sending tradeflow from ws");
                                    }
                                }
                                TradeTicks::WebsocketSuccessTick(s) => {
                                    if s.ret_msg == "pong" {
                                        // println!(
                                        //     "connection ping trade = {}",
                                        //     ping_timer.elapsed().as_millis()
                                        // );
                                    }
                                }
                            }

                            // REGULAR UPDATE
                            // println!("Time since last tick: {}", websocket_timer.elapsed().as_micros());

                            if txt.contains("topic") {
                            } else if txt.contains("pong") {
                            }
                        }
                        Message::Binary(_) => todo!(),
                        Message::Ping(_) => {
                            write
                                .send(Message::Pong(vec![]))
                                .await
                                .expect("ponging went wrong brah");
                            println!("Pong sent from ping");
                        }
                        Message::Pong(_) => {
                            println!("Pong received for some reason")
                        }
                        Message::Close(a) => {
                            let frm = a.expect("something went wrong getting the reason info");
                            println!(
                                "The server shut down on us because of {}, code: {}",
                                frm.reason, frm.code
                            );
                            write
                                .close()
                                .await
                                .expect("somehow closing the stream messed up");
                        }
                        Message::Frame(_) => todo!(),
                    }
                }
                WebsocketMessager::Ping() => {
                    let ping_load = serde_json::to_string(&WebsocketPing {
                        message_type: "ping".to_string(),
                    })
                    .expect("something went wrong stringifying ping query json");
                    // println!("Trade ping");
                    write
                        .send(Message::Text(ping_load))
                        .await
                        .expect("sending ping went wrong");
                }
            },
            None => {}
        }
    }
}