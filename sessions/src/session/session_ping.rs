use crate::actors::SessionPing;
use crate::rpc::{ResponseParamsSuccess, RpcResponsePayload};
use crate::ClientSession;
use xtra::prelude::*;

impl Handler<SessionPing> for ClientSession {
    type Return = RpcResponsePayload;

    async fn handle(&mut self, _message: SessionPing, ctx: &mut Context<Self>) -> Self::Return {
        RpcResponsePayload::Success(ResponseParamsSuccess::SessionPing(true))
    }
}
