use tokio::sync::mpsc::{Sender, Receiver, channel};
use std::time::{Instant, Duration};
use std::time::{SystemTime};
use hmac::Mac;

use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use futures::{SinkExt, StreamExt};
use tokio::runtime::Runtime;

use crate::HmacSha256;
use crate::backend::bybit::stream::{BybitStream, BybitTimeTick, WebsocketPing, WSPrivateTicks, RestWallet};
use crate::backend::bybit::stream::Signal;
use crate::backend::bybit::stream::ArgType;
use crate::backend::bybit::stream::WebsocketMessager;
use crate::backend::bybit::stream::WebsocketSubscribe;

use crate::config::CONFIG;

/// Connects the stream to a private type websocket and emits signals to the provided sender.
/// NOTE: Should be called from a new thread to avoid blocking the main thread.
pub async fn connect_private(sender: Sender<Signal>) {
    let stream = BybitStream::new();
    let mut ping_timer = Instant::now();

    let (ws_stream, _res) = connect_async(CONFIG.bybit_perpetuals_private_url.clone())
        .await
        .expect("error building stream");
    let (mut write, mut read) = ws_stream.split();
    //AUTHORIZE
    let expires = (SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
        + 30000)
        .to_string();
    let mut mac = HmacSha256::new_from_slice(CONFIG.bybit_secret.as_bytes()).expect("N");
    mac.update(format!("GET/realtime{}", expires).as_bytes());
    let signature: String = format!("{:X}", mac.finalize().into_bytes());
    let prisub = serde_json::to_string(&WebsocketSubscribe {
        message_type: "auth".to_string(),
        args: vec![
            ArgType::String(CONFIG.bybit_key.clone()),
            ArgType::String(expires),
            ArgType::String(signature),
        ],
    })
    .expect("something went wrong stringifying query json");
    // println!("{}", prisub);
    write
        .send(Message::Text(prisub))
        .await
        .expect("Auth went wrong");

    // CHANNEL SETUP
    let prisub = serde_json::to_string(&WebsocketSubscribe {
        message_type: "subscribe".to_string(),
        args: vec![
            ArgType::String("execution".to_string()),
            ArgType::String("position".to_string()),
            ArgType::String("order".to_string()),
            ArgType::String("stop_order".to_string()),
            ArgType::String("wallet".to_string()),
        ],
    })
    .expect("something went wrong stringifying exec queryjson");
    write
        .send(Message::Text(prisub))
        .await
        .expect("initialization of executions went wrong");

    let heartbeat_timer = Instant::now();
    let (send, mut rec): (Sender<WebsocketMessager>, Receiver<WebsocketMessager>) = channel(1);
    let ping_send = send.clone();
    let msg_send = send.clone();
    let init_send = send.clone();

    let mut ping_interval = tokio::time::interval(Duration::from_secs(25));

    let rt = Runtime::new().expect("Failed to create runtime");
    rt.spawn(async move {
        loop {
            ping_interval.tick().await;
            if ping_send.send(WebsocketMessager::Ping()).await.is_err() {
                panic!("something went wrong sending private ping to main ws thread");
            }
        }
    });
    rt.spawn(async move {
        loop {
            let msg = read
                .next()
                .await
                .expect("something went wrong waiting for next message")
                .expect("something went wrong getting next message");
            if msg_send.send(WebsocketMessager::Message(msg)).await.is_err() {
                panic!("something went wrong sending private msg to main ws thread");
            }
        }
    });

    rt.spawn(async move {
        let before_call = Instant::now();
        let time_rest =
            reqwest::blocking::get(format!("{}/v2/public/time", CONFIG.bybit_rest_url))
                .expect("something went wrong getting wallet rest");
        let latency = before_call.elapsed().as_millis() / 2;
        // println!("Latency: {}", latency);
        let server_time = serde_json::from_str::<BybitTimeTick>(
            &time_rest
                .text()
                .expect("something went wrong getting time query text"),
        )
        .expect("something went wrong serdeing time query");
        let expires = {
            let expires_fl: f64 = &server_time
                .time_now
                .parse()
                .expect("something went wrong converting time to float")
                * 1000.0;
            expires_fl.round() as u128
        };
        let mut mac = HmacSha256::new_from_slice(CONFIG.bybit_secret.as_bytes()).expect("N");
        mac.update(format!("api_key={}&timestamp={}", CONFIG.bybit_key, expires).as_bytes());
        let signature: String = format!("{:X}", mac.finalize().into_bytes());
        let wallet_rest = reqwest::blocking::get(
                &format!(
                    "{}/v2/private/wallet/balance?api_key={}&timestamp={}&sign={}",
                    CONFIG.bybit_rest_url, CONFIG.bybit_key, expires, signature
                ),
        )
        .expect("something went wrong getting wallet rest");
        let parsed_wallet = serde_json::from_str::<RestWallet>(
            &wallet_rest
                .text()
                .expect("something went wrong getting wallet text"),
        )
        .expect("something went wrong parsing wallet rest text");
    });

    loop {
        match rec.recv().await {
            Some(m) => {
                match m {
                    WebsocketMessager::Message(msg) => {
                        match msg {
                            Message::Text(txt) => {
                                // println!("[DEBUG] [PRIVATE_STREAM] {:?}", txt);
                                let tick =
                                    serde_json::from_str::<WSPrivateTicks>(&txt.to_string())
                                        .expect("something went wrong in the tick enum serde");

                                match tick {
                                    WSPrivateTicks::PrivateTicks(pt) => {
                                        if sender.send(Signal::PrivateTicks(pt)).await.is_err() {
                                            panic!("something went wrong sending private tick");
                                        }
                                    }
                                    WSPrivateTicks::WebsocketSuccessTick(s) => {
                                        if s.ret_msg == "pong" {
                                            // println!(
                                            //     "connection ping private = {}",
                                            //     ping_timer.elapsed().as_millis()
                                            // );
                                        }
                                    }
                                }
                            }
                            Message::Binary(_) => {
                                print!("recieved binary for some reason")
                            }
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
                                let frm =
                                    a.expect("something went wrong getting the reason info");
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
                        ping_timer = Instant::now();
                        // println!("Ping sent from private timer");
                    }
                }
            }
            None => {}
        }
    }
}