use std::thread;
use std::time::{Instant, Duration, UNIX_EPOCH, SystemTime};

use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use futures::{SinkExt, StreamExt};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use tokio::task;
// use tokio::sync::mpsc::{Sender, Receiver, channel};
use tokio::time;

use crate::backend::binance::types::{WebsocketSubscribe, Signal, TradeFlows, FuturesTrades, StreamWrapper, UserDataStreams, UserStreamWrapper, OrderUpdateData, PositionUpdateData, StreamExpired, WebsocketMessager};
use crate::config::CONFIG;
use crate::strategy::binance::{StrategyMessage, AccountMessage};

pub async fn connect_user_data(sender: crossbeam_channel::Sender<StrategyMessage>) {
    let key = get_key().await;
    let url = format!("{}/ws/{}", CONFIG.binance_perpetuals_url.clone(), key);
    info!("url {}", url);
    let (ws_stream, res) = connect_async(url)
        .await
        .expect("error building stream");
    if !(res.status().is_informational() || res.status().is_success()) { panic!("panic stream: {:?}\nres: {:?}", ws_stream, res); }

    let (mut write, mut read) = ws_stream.split();

    let (send, mut rec): (tokio::sync::mpsc::Sender<WebsocketMessager>, tokio::sync::mpsc::Receiver<WebsocketMessager>) = tokio::sync::mpsc::channel(1);
    let ping_send = send.clone();
    let msg_send = send.clone();

    let rt = Runtime::new().expect("Failed to create runtime");
    // The user data stream doesn't send its own pings, so we'll need a timer of our own
    rt.spawn(async move {
        let mut interval = time::interval(Duration::from_secs(270));
        loop {
            interval.tick().await;
            if ping_send.send(WebsocketMessager::Ping()).await.is_err() {
                panic!("something went wrong sending private ping to main ws thread");
            }
        }
    });
    rt.spawn(async move {
        let mut interval = time::interval(Duration::from_secs(3300));
        loop {
            interval.tick().await;
            get_key().await;
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
            info!("msg loop ud: {}", msg);
            if msg_send.send(WebsocketMessager::Message(msg)).await.is_err() {
                panic!("something went wrong sending private msg to main ws thread");
            }
        }
    });

    loop {
        match rec.recv().await.unwrap() {
            WebsocketMessager::Message(m) => {
                match m {

                    Message::Text(txt) => {
                        info!("ud stream txt {}", txt);
                        if txt.contains("ORDER_TRADE_UPDATE") {
                            let ud = serde_json::from_str::<UserStreamWrapper<OrderUpdateData>>(&txt.to_string()).expect("Deser UD went wrong");
                            sender.send(StrategyMessage::AccountMessage(AccountMessage::OrderUpdate(ud.data))).expect("err sending od out of ws");

                        } else if txt.contains("ACCOUNT_UPDATE") {
                            let ud = serde_json::from_str::<UserStreamWrapper<PositionUpdateData>>(&txt.to_string()).expect("Deser UD went wrong");
                            sender.send(StrategyMessage::AccountMessage(AccountMessage::PositionUpdate(ud.data))).expect("err sending pd out of ws");

                        } else if txt.contains("listenKeyExpired") {
                            let ud = serde_json::from_str::<StreamExpired>(&txt.to_string()).expect("Deser UD went wrong");
                            panic!("{:?}", ud);

                        } else if txt.contains("MARGIN_CALL") {
                            todo!();
                        } else if txt.contains("ACCOUNT_CONFIG_UPDATE") {
                            todo!();
                        } else {
                            panic!("Couldn't match incoming UD");
                        }
                    },
                    Message::Binary(_) => todo!(),
                    Message::Ping(_) => {
                        write.send(Message::Pong(vec![]))
                            .await.expect("UD error attempting pong");
                    },
                    Message::Pong(_) => info!("Pong received for some reason"),
                    Message::Close(a) => {
                        let frm = a.expect("something went wrong getting the reason info");
                        println!(
                            "The server shut down the UD stream because of {}, code: {}",
                            frm.reason, frm.code
                        );
                        write
                            .close()
                            .await
                            .expect("somehow closing the stream messed up");
                    },
                    Message::Frame(_) => todo!(),
                }
            },
            WebsocketMessager::Ping() => {
                write.send(Message::Pong(vec![]))
                    .await.expect("UD error attempting pong");
            },
        }
    }
}

async fn get_key() -> String {
    let client = reqwest::Client::builder().https_only(true).pool_max_idle_per_host(4).pool_idle_timeout(None).use_rustls_tls().build().expect("msg");
    let req = serde_json::to_string(&KeyRequest {
        // timestamp: SystemTime::now().duration_since(UNIX_EPOCH).expect("msg").as_millis(),
        signature: CONFIG.binance_secret.clone(),
    }).expect("msg");
    let key = client
        .post(format!("{}/fapi/v1/listenKey", CONFIG.binance_rest_url))
        .header("X-MBX-APIKEY", CONFIG.binance_key.clone())
        .body(req)
        .send()
        .await
        .expect("error receiving key response")
        .text()
        .await
        .expect("error getting key response text");
    info!("ud key: {}", key);
    serde_json::from_str::<Key>(&key).expect("error deserializing listen key res").listen_key
}

#[derive(Deserialize)]
struct Key {
    #[serde(rename = "listenKey")]
    pub listen_key: String,
}

#[derive(Serialize)]
pub struct KeyRequest {
    pub signature: String,
}