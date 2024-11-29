use {
    crate::{
        rpc::{
            ResponseParamsError,
            ResponseParamsSuccess,
            RpcResponsePayload,
            SessionRequestRequest,
        },
        ClientSession,
        SessionHandler,
        WalletRequestResponse,
    },
    xtra::prelude::*,
};

impl Handler<SessionRequestRequest> for ClientSession {
    type Return = RpcResponsePayload;

    async fn handle(
        &mut self,
        message: SessionRequestRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        let l = self.handler.lock().await;
        let result = l.request(message).await;
        match result {
            WalletRequestResponse::Success(v) => {
                RpcResponsePayload::Success(ResponseParamsSuccess::SessionRequest(v))
            }
            WalletRequestResponse::Error(e) => {
                RpcResponsePayload::Error(ResponseParamsError::SessionRequest(e.into()))
            }
        }
    }
}
