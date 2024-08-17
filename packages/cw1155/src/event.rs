use cosmwasm_std::{attr, Response, Uint128};
use cw_utils::Event;

/// Tracks token transfer/mint/burn actions
pub struct TransferEvent {
    pub from: Option<String>,
    pub to: Option<String>,
    pub token_id: String,
    pub amount: Uint128,
}

impl Event for TransferEvent {
    fn add_attributes(&self, rsp: &mut Response) {
        rsp.attributes.push(attr("action", "transfer"));
        rsp.attributes.push(attr("token_id", self.token_id.clone()));
        rsp.attributes.push(attr("amount", self.amount));
        if let Some(from) = self.from.clone() {
            rsp.attributes.push(attr("from", from));
        }
        if let Some(to) = self.to.clone() {
            rsp.attributes.push(attr("to", to));
        }
    }
}

/// Tracks approve_all status changes
pub struct ApproveAllEvent<'a> {
    pub sender: &'a String,
    pub operator: &'a String,
    pub approved: bool,
}

impl<'a> Event for ApproveAllEvent<'a> {
    fn add_attributes(&self, rsp: &mut Response) {
        rsp.attributes.push(attr("action", "approve_all"));
        rsp.attributes.push(attr("sender", self.sender));
        rsp.attributes.push(attr("operator", self.operator));
        rsp.attributes
            .push(attr("approved", (self.approved as u32).to_string()));
    }
}
