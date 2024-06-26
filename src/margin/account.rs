use crate::util::*;
use crate::model::*;
use crate::client::*;
use crate::errors::*;
use std::collections::BTreeMap;
use crate::account::{OrderSide, OrderType, TimeInForce};
use crate::api::{API, Margin};
use crate::margin::SideEffectType;

#[derive(Clone)]
pub struct MarginAccount {
    pub client: Client,
    pub is_isolated: bool,
    pub recv_window: u64,
}


impl MarginAccount {

    // Account Information
    pub fn get_account(&self) -> Result<MarginAccountInformation> {
        let request = build_signed_request(BTreeMap::new(), self.recv_window)?;
        self.client
            .get_signed(API::Margin(Margin::Account), Some(request))
    }

    // Account Information
    pub fn get_isolated_account(&self) -> Result<IsoMarginAccountInformation> {
        let request = build_signed_request(BTreeMap::new(), self.recv_window)?;
        self.client
            .get_signed(API::Margin(Margin::AccountIsolated), Some(request))
    }

    // Current open orders for ONE symbol
    pub fn get_open_orders<S>(&self, symbol: S) -> Result<Vec<Order>>
    where
        S: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("symbol".into(), symbol.into());
        parameters.insert("isIsolated".into(), bool_to_string(self.is_isolated));

        let request = build_signed_request(parameters, self.recv_window)?;
        self.client
            .get_signed(API::Margin(Margin::OpenOrders), Some(request))
    }

    // All current open orders
    pub fn get_all_open_orders(&self) -> Result<Vec<Order>> {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("isIsolated".into(), bool_to_string(self.is_isolated));

        let request = build_signed_request(parameters, self.recv_window)?;
        self.client
            .get_signed(API::Margin(Margin::OpenOrders), Some(request))
    }


    // Current open orders for ONE symbol
    pub fn get_all_orders<S>(&self, symbol: S, limit: usize) -> Result<Vec<Order>>
    where
        S: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("symbol".into(), symbol.into());
        parameters.insert("isIsolated".into(), bool_to_string(self.is_isolated));
        parameters.insert("limit".into(), limit.to_string());

        let request = build_signed_request(parameters, self.recv_window)?;
        self.client
            .get_signed(API::Margin(Margin::AllOrders), Some(request))
    }

    pub fn get_order_status<S>(&self, symbol: S, client_order_id: S) -> Result<Order>
    where
        S: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("symbol".into(), symbol.into());
        parameters.insert("isIsolated".into(), bool_to_string(self.is_isolated));
        parameters.insert("origClientOrderId".into(), client_order_id.into());
        let request = build_signed_request(parameters, self.recv_window)?;
        self.client
            .get_signed(API::Margin(Margin::Order), Some(request))
    }




    /// Place a custom order
    #[allow(clippy::too_many_arguments)]
    pub fn custom_order<S>(
        &self, symbol: S, qty: f64, price: f64, stop_price: Option<f64>, order_side: OrderSide,
        order_type: OrderType, time_in_force: TimeInForce,  iceberg_qty: Option<f64>, new_client_order_id: Option<String>,
        side_effect: SideEffectType
    ) -> Result<Transaction>
    where
        S: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("symbol".into(), symbol.into());
        parameters.insert("isIsolated".into(), bool_to_string(self.is_isolated));
        parameters.insert("sideEffectType".into(), side_effect.into());
        parameters.insert("side".into(), order_side.into());
        parameters.insert("type".into(), order_type.into());
        parameters.insert("quantity".into(), qty.to_string());

        if let Some(stop_price) = stop_price {
            parameters.insert("stopPrice".into(), stop_price.to_string());
        }

        if price != 0.0 {
            parameters.insert("price".into(), price.to_string());
            parameters.insert("timeInForce".into(), time_in_force.into());
        }

        if let Some(client_order_id) = new_client_order_id {
            parameters.insert("newClientOrderId".into(), client_order_id);
        }

         if let Some(iceberg) = iceberg_qty {
            parameters.insert("icebergQty".into(), iceberg.to_string());
        }

        let request = build_signed_request(parameters, self.recv_window)?;
        self.client.post_signed(API::Margin(Margin::Order), request)
    }




    pub fn cancel_order_with_client_id<S>(
        &self, symbol: S, orig_client_order_id: String,
    ) -> Result<OrderCanceled>
    where
        S: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("symbol".into(), symbol.into());
        parameters.insert("origClientOrderId".into(), orig_client_order_id);
        parameters.insert("isIsolated".into(), bool_to_string(self.is_isolated));


        let request = build_signed_request(parameters, self.recv_window)?;
        self.client
            .delete_signed(API::Margin(Margin::Order), Some(request))
    }
}
