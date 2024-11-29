use {
    crate::{
        actors::SessionPing,
        rpc::{ResponseParamsSuccess, RpcResponsePayload},
        ClientSession,
    },
    xtra::prelude::*,
};

impl Handler<SessionPing> for ClientSession {
    type Return = RpcResponsePayload;

    async fn handle(&mut self, _message: SessionPing, _ctx: &mut Context<Self>) -> Self::Return {
        RpcResponsePayload::Success(ResponseParamsSuccess::SessionPing(true))
    }
}
