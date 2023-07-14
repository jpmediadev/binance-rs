
use crate::errors::*;
use crate::config::*;
use crate::model::*;
use url::Url;
use serde::{Deserialize, Serialize};

use std::sync::atomic::{AtomicBool, Ordering};
use std::net::TcpStream;
use std::time::Duration;
use native_tls::{TlsConnector, TlsStream};
use serde_json::json;
use tungstenite::{client, Message};
use tungstenite::protocol::WebSocket;
use tungstenite::handshake::client::Response;

#[allow(clippy::all)]
enum WebsocketAPI {
    Default,
    MultiStream,
    Custom(String),
}

impl WebsocketAPI {
    fn params(self, subscription: &str) -> String {
        match self {
            WebsocketAPI::Default => format!("wss://stream.binance.com:9443/ws/{}", subscription),
            WebsocketAPI::MultiStream => format!(
                "wss://stream.binance.com:9443/stream?streams={}",
                subscription
            ),
            WebsocketAPI::Custom(url) => format!("{}/{}", url, subscription),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WebsocketEvent {
    AccountUpdate(AccountUpdateEvent),
    BalanceUpdate(BalanceUpdateEvent),
    OrderTrade(OrderTradeEvent),
    AggrTrades(AggrTradesEvent),
    Trade(TradeEvent),
    OrderBook(OrderBook),
    DayTicker(DayTickerEvent),
    DayTickerAll(Vec<DayTickerEvent>),
    Kline(KlineEvent),
    DepthOrderBook(DepthOrderBookEvent),
    BookTicker(BookTickerEvent),
}

pub struct WebSockets<'a> {
    pub socket: Option<(WebSocket<TlsStream<TcpStream>>, Response)>,
    handler: Box<dyn FnMut(WebsocketEvent) -> Result<()> + 'a>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Events {
    Vec(Vec<DayTickerEvent>),
    BalanceUpdateEvent(BalanceUpdateEvent),
    DayTickerEvent(DayTickerEvent),
    BookTickerEvent(BookTickerEvent),
    AccountUpdateEvent(AccountUpdateEvent),
    OrderTradeEvent(OrderTradeEvent),
    AggrTradesEvent(AggrTradesEvent),
    TradeEvent(TradeEvent),
    KlineEvent(KlineEvent),
    OrderBook(OrderBook),
    DepthOrderBookEvent(DepthOrderBookEvent),
}

impl<'a> WebSockets<'a> {
    pub fn new<Callback>(handler: Callback) -> WebSockets<'a>
    where
        Callback: FnMut(WebsocketEvent) -> Result<()> + 'a,
    {
        WebSockets {
            socket: None,
            handler: Box::new(handler),
        }
    }

    pub fn connect(&mut self, subscription: &str) -> Result<()> {
        self.connect_wss(WebsocketAPI::Default.params(subscription))
    }

    pub fn connect_with_config(&mut self, subscription: &str, config: &Config) -> Result<()> {
        self.connect_wss(WebsocketAPI::Custom(config.ws_endpoint.clone()).params(subscription))
    }

    pub fn connect_multiple_streams(&mut self, endpoints: &[String]) -> Result<()> {
        self.connect_wss(WebsocketAPI::MultiStream.params(&endpoints.join("/")))
    }

    fn connect_wss(&mut self, wss: String) -> Result<()> {
        let url = Url::parse(&wss)?;
        let host = url.host_str().unwrap();
        let port = url.port().unwrap_or(443);
        let connector = TlsConnector::new()?;
        let tcp_stream = TcpStream::connect((host, port))?;
        tcp_stream.set_read_timeout(Some(Duration::from_secs(10)))?; // Установите желаемый таймаут
        let tls_stream = connector.connect(host, tcp_stream)?;
        let (socket, response) = client(url, tls_stream)?;
        self.socket = Some((socket, response));
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(ref mut socket) = self.socket {
            socket.0.close(None)?;
            return Ok(());
        }
        bail!("Not able to close the connection");
    }

    pub fn test_handle_msg(&mut self, msg: &str) -> Result<()> {
        self.handle_msg(msg)
    }

    fn handle_msg(&mut self, msg: &str) -> Result<()> {
        let mut value: serde_json::Value = serde_json::from_str(msg)?;


        if let Some(stream_value) = value.get("stream") {
            if let Some(stream_str) = stream_value.as_str() {
                let stream = stream_str.to_owned();
                let parts: Vec<&str> = stream.split('@').collect();
                if parts.len() >= 1 {
                    let symbol = parts[0].to_uppercase();
                    if let Some(data) = value.get_mut("data") {
                        if data.is_object() {
                            let data_obj = data.as_object_mut().unwrap();
                            data_obj.insert("symbol".to_string(), json!(symbol));
                        }
                        self.handle_msg(&data.to_string())?;
                        return Ok(());
                    }
                }
            }
        }


        if let Ok(events) = serde_json::from_value::<Events>(value) {
            let action = match events {
                Events::Vec(v) => WebsocketEvent::DayTickerAll(v),
                Events::BookTickerEvent(v) => WebsocketEvent::BookTicker(v),
                Events::BalanceUpdateEvent(v) => WebsocketEvent::BalanceUpdate(v),
                Events::AccountUpdateEvent(v) => WebsocketEvent::AccountUpdate(v),
                Events::OrderTradeEvent(v) => WebsocketEvent::OrderTrade(v),
                Events::AggrTradesEvent(v) => WebsocketEvent::AggrTrades(v),
                Events::TradeEvent(v) => WebsocketEvent::Trade(v),
                Events::DayTickerEvent(v) => WebsocketEvent::DayTicker(v),
                Events::KlineEvent(v) => WebsocketEvent::Kline(v),
                Events::OrderBook(v) => WebsocketEvent::OrderBook(v),
                Events::DepthOrderBookEvent(v) => WebsocketEvent::DepthOrderBook(v),
            };
            (self.handler)(action)?;
        }
        Ok(())
    }

    pub fn event_loop(&mut self, should_stop: &AtomicBool) -> Result<()> {
        let mut ping_counter = 0;

        while !should_stop.load(Ordering::Relaxed) {
            if let Some(ref mut socket) = self.socket {
                let message = socket.0.read_message();
                match message {
                    Ok(message) => match message {
                        Message::Text(msg) => {
                            if let Err(e) = self.handle_msg(&msg) {
                                bail!(format!("Error on handling stream message: {}", e));
                            }
                        }
                        Message::Ping(_) => {
                            socket.0.write_message(Message::Pong(vec![])).unwrap();
                        }
                        Message::Pong(_) => {
                            ping_counter = 0;
                        }
                        Message::Binary(_) => (),
                        Message::Close(e) => bail!(format!("Disconnected {:?}", e)),
                    },
                    Err(error) => {
                        // Таймаут истек; вы можете обработать эту ситуацию, например, закрыть соединение
                        // отправляем 3 пинга если нет ответа - ошибка
                        if let Err(err) = socket.0.write_message(Message::Ping(vec![])){
                             bail!(format!("Disconnected loop is dead {err:?} {error:?}"));
                        };
                        ping_counter += 1;

                        if ping_counter >= 3{
                            bail!(format!("Disconnected loop is dead {error}"));
                        }
                    }
                }
            }
        }
        bail!("running loop closed");
    }
}
