use crate::actors::RequestHandlerActor;
use crate::rpc::{
    ErrorParams, IntoUnknownError, PairDeleteRequest, RpcResponse, RpcResponsePayload,
};
use crate::{MessageId, PairingManager, Result, Topic};
use tracing::warn;

impl RequestHandlerActor {
    pub(super) fn send_response(&self, resp: RpcResponse) {
        let me = self.clone();
        let id = resp.id.clone();
        let topic = resp.topic.clone();
        tokio::spawn(async move {
            if let Err(err) = me.responder.send(resp).await {
                warn!(
                    "Failed to send response for id {} on topic {} {}",
                    id, topic, err
                );
            }
        });
    }

    async fn _handle_pair_request<M>(&self, id: MessageId, topic: Topic, request: M) -> Result<()>
    where
        M: Send + 'static,
        PairingManager: xtra::Handler<M>,
        <PairingManager as xtra::Handler<M>>::Return: Into<RpcResponsePayload>,
    {
        let mgr = self.pair_managers.as_ref().ok_or(crate::Error::NoPairManager(topic.clone()))?;
        let response: RpcResponse = mgr.send(request).await.map(|r| RpcResponse {
            id,
            topic: topic.clone(),
            payload: r.into(),
        })?;
        self.send_response(response);
        Ok(())
    }

    pub(super) async fn handle_pair_mgr_request<M>(&self, id: MessageId, topic: Topic, request: M)
    where
        M: IntoUnknownError + Send + 'static,
        PairingManager: xtra::Handler<M>,
        <PairingManager as xtra::Handler<M>>::Return: Into<RpcResponsePayload>,
    {
        let u: RpcResponse = RpcResponse::unknown(id, topic.clone(), (&request).unknown());
        if let Err(e) = self._handle_pair_request(id, topic, request).await {
            warn!("failed to get response from pair manager: '{e}'");
            self.send_response(u);
        }
    }
}
