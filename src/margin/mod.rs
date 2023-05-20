pub mod account;


pub enum SideEffectType{
    NoSideEffect,
    MarginBuy,
    AutoRepay
}

impl Default for SideEffectType{
    fn default() -> Self {
        SideEffectType::NoSideEffect
    }
}


impl From<SideEffectType> for String {
    fn from(item: SideEffectType) -> Self {
        match item {
            SideEffectType::NoSideEffect => String::from("NO_SIDE_EFFECT"),
            SideEffectType::MarginBuy => String::from("MARGIN_BUY"),
            SideEffectType::AutoRepay => String::from("AUTO_REPAY"),
        }
    }
}