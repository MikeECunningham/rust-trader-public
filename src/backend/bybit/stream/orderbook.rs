use std::time::{Instant, Duration};

use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use futures::{SinkExt, StreamExt};
use tokio::task;
use tokio::sync::mpsc::{Sender, Receiver, channel};
use tokio::time;

use crate::backend::bybit::stream::{BybitStream, WebsocketPing, OrderBookTicks, OBTick, OBTickData};
use crate::backend::bybit::stream::Signal;
use crate::backend::bybit::stream::ArgType;
use crate::backend::bybit::stream::WebsocketMessager;
use crate::backend::bybit::stream::WebsocketSubscribe;

use crate::config::CONFIG;


/// Connects the stream to a orderbook type websocket and emits signals to the provided sender.
/// NOTE: Should be called from a new thread to avoid blocking the main thread.
pub async fn connect_orderbook(sender: Sender<Signal>, symbol: String) {
    let stream = BybitStream::new();
    // WEBSOCKET SETUP
    let (ws_stream, _res) = connect_async(CONFIG.bybit_perpetuals_url.clone())
        .await
        .expect("error building stream");

    let (mut write, mut read) = ws_stream.split();
    // CHANNEL SETUP
    // COMPOSE AND STRINGIFY THE REQUEST
    let obsub = serde_json::to_string(&WebsocketSubscribe {
        message_type: "subscribe".to_string(),
        args: vec![ArgType::String(format!("orderBook_200.100ms.{}", symbol))],
    })
    .expect("something went wrong stringifying query json");
    // SEND THE REQUEST
    write
        .send(Message::Text(obsub))
        .await
        .expect("initialization of the orderbook went wrong");
    let mut heartbeat_timer = Instant::now();

    let (send, mut rec): (Sender<WebsocketMessager>, Receiver<WebsocketMessager>) = channel(1);
    let ping_send = send.clone();
    let msg_send = send.clone();

    task::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(25000));
        loop {
            interval.tick().await;

            ping_send
                .send(WebsocketMessager::Ping())
                .await;
            heartbeat_timer = Instant::now();
        }
    });
    task::spawn(async move {
        loop {
            let msg = read
                .next()
                .await
                .expect("something went wrong waiting for next message")
                .expect("something went wrong getting next message");
            msg_send
                .send(WebsocketMessager::Message(msg))
                .await;
        }
    });

    // BEGIN TO LISTEN
    loop {
        match rec.recv().await {
            Some(m) => match m {
                WebsocketMessager::Message(msg) => {
                    match msg {
                        Message::Text(txt) => {
                            let ob = serde_json::from_str::<OrderBookTicks>(&txt.to_string())
                                .expect("Deserializing OB went wrong");
                            match ob {
                                OrderBookTicks::OBTick(delta) => {
                                    if stream.orderbook_activated {
                                        sender.send(Signal::Orderbook(delta)).await;
                                    }
                                }
                                OrderBookTicks::BybitOBInit(snapshot) => {
                                    if stream.orderbook_activated {
                                        // CONVERT THE SNAPSHOT TICK INTO STANDARD ORDERBOOK UPDATE FORM
                                        let refresh = OBTick {
                                            message_type: snapshot.message_type,
                                            channel: snapshot.channel,
                                            timestamp: snapshot.timestamp,
                                            cross_seq: snapshot.cross_seq,
                                            data: OBTickData {
                                                delete: vec![],
                                                update: vec![],
                                                insert: snapshot.data.order_book,
                                            },
                                        };
                                        // SEND SNAPSHOT UPDATE TO MODEL THREAD
                                        sender
                                        .send(Signal::Orderbook(refresh))
                                        .await.expect("something went wrong sending the init ob out of the ws");
                                        // println!("ORDERFLOW SNAPSHOT TIME: {}", model_timer.elapsed().as_micros());
                                    }
                                }
                                OrderBookTicks::WebsocketSuccessTick(s) => {
                                    if s.ret_msg == "pong" {
                                        // println!(
                                        //     "connection ping ob = {}",
                                        //     ping_timer.elapsed().as_millis()
                                        // );
                                    }
                                }
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
                    write
                        .send(Message::Text(ping_load))
                        .await
                        .expect("sending ping went wrong");
                }
            },
            None => {}
        }
        // MESSAGE RECEIVED, BEGIN EXAMINATION
    }
}