use crate::api::API;
use crate::model::*;
use crate::client::*;
use crate::errors::*;


#[derive(Clone)]
pub struct UserStream {
    pub client: Client,
    pub recv_window: u64,
    pub endpoint: API
}

impl UserStream {
    // User Stream
    pub fn start(&self) -> Result<UserDataStream> {
        self.client.post(self.endpoint.clone())
    }

    // Current open orders on a symbol
    pub fn keep_alive(&self, listen_key: &str) -> Result<Success> {
        self.client.put(self.endpoint.clone(), listen_key)
    }

    pub fn close(&self, listen_key: &str) -> Result<Success> {
        self.client
            .delete(self.endpoint.clone(), listen_key)
    }
}


