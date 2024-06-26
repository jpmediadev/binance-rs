
use std::net::TcpStream;
use serde::Deserialize;
use crate::errors;
use tungstenite::ClientHandshake;
use native_tls::TlsStream;
use crate::model::CancelReplace;


#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum BinanceContent {
    CancelReplace{
        code: i16,
        msg: String,
        data: CancelReplace
    },
    Error(BinanceContentError)
}

#[derive(Debug, Deserialize)]
pub struct BinanceContentError {
    pub code: i16,
    pub msg: String,

}

error_chain! {
    errors {

        CancelReplaceError(data: CancelReplace)

        BinanceError(response: BinanceContentError)

        KlineValueMissingError(index: usize, name: &'static str) {
            description("invalid Vec for Kline"),
            display("{} at {} is missing", name, index),
        }
        TlsConnectorError(error: native_tls::Error) {
            description("TLS connector error"),
            display("TLS connector error: {}", error),
        }

        HandshakeError(error: native_tls::HandshakeError<TcpStream>) {
            description("TLS handshake error"),
            display("TLS handshake error: {}", error),
        }
         WebSocketHandshakeError(error: tungstenite::HandshakeError<ClientHandshake<TlsStream<TcpStream>>>) {
            description("WebSocket handshake error"),
            display("WebSocket handshake error: {}", error),
        }
     }

    foreign_links {
        ReqError(reqwest::Error);
        InvalidHeaderError(reqwest::header::InvalidHeaderValue);
        IoError(std::io::Error);
        ParseFloatError(std::num::ParseFloatError);
        UrlParserError(url::ParseError);
        Json(serde_json::Error);
        Tungstenite(tungstenite::Error);
        TimestampError(std::time::SystemTimeError);
    }
}

impl From<native_tls::Error> for errors::Error {
    fn from(err: native_tls::Error) -> errors::Error {
        errors::Error::from(errors::ErrorKind::TlsConnectorError(err))
    }
}

impl From<native_tls::HandshakeError<TcpStream>> for errors::Error {
    fn from(err: native_tls::HandshakeError<TcpStream>) -> errors::Error {
        errors::Error::from(errors::ErrorKind::HandshakeError(err))
    }
}

impl From<tungstenite::HandshakeError<tungstenite::ClientHandshake<native_tls::TlsStream<std::net::TcpStream>>>> for errors::Error {
    fn from(err: tungstenite::HandshakeError<tungstenite::ClientHandshake<native_tls::TlsStream<std::net::TcpStream>>>) -> errors::Error {
        errors::Error::from(errors::ErrorKind::WebSocketHandshakeError(err))
    }
}

impl Error {
    pub fn cancel_replace(&self) -> Option<&CancelReplace> {
        match &self.0 {
            ErrorKind::CancelReplaceError(cancel_replace) => Some(cancel_replace),
            _ => None,
        }
    }
}