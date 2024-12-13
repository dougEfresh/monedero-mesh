use {
    crate::{
        actors::{SessionRequestHandlerActor, Unsubscribe},
        rpc::{IntoUnknownError, RpcResponse, RpcResponsePayload},
        ClientSession,
        Result,
        Topic,
    },
    monedero_domain::MessageId,
    tracing::warn,
};

impl SessionRequestHandlerActor {
    pub(super) async fn send_response(&self, resp: RpcResponse) {
        let id = resp.id;
        let topic = resp.topic.clone();
        if let Err(err) = self.responder.send(resp).await {
            warn!(
                "Failed to send response for id {} on topic {} {}",
                id, topic, err
            );
        }
    }

    #[allow(clippy::significant_drop_tightening)]
    async fn internal_handle_session_request<M>(
        &self,
        id: MessageId,
        topic: Topic,
        request: M,
    ) -> Result<()>
    where
        M: IntoUnknownError + Send + 'static,
        ClientSession: xtra::Handler<M>,
        <ClientSession as xtra::Handler<M>>::Return: Into<RpcResponsePayload>,
    {
        let mgr = self
            .sessions
            .get(&topic)
            .ok_or(crate::Error::NoClientSession(topic.clone()))?;
        let response: RpcResponse = mgr.send(request).await.map(|r| RpcResponse {
            id,
            topic: topic.clone(),
            payload: r.into(),
        })?;
        self.send_response(response).await;
        Ok(())
    }

    pub(super) async fn handle_session_delete(&self, topic: Topic) {
        self.sessions.remove(&topic);
        if let Err(e) = self.responder.send(Unsubscribe(topic.clone())).await {
            warn!("failed to unsubscribe to {topic} '{e}'");
        }
        let _ = self.cipher.delete_session(&topic);
    }

    pub(super) async fn handle_session_request<M>(&self, id: MessageId, topic: Topic, request: M)
    where
        M: IntoUnknownError + Send + 'static,
        ClientSession: xtra::Handler<M>,
        <ClientSession as xtra::Handler<M>>::Return: Into<RpcResponsePayload>,
    {
        let u: RpcResponse = RpcResponse::unknown(id, topic.clone(), request.unknown());
        if let Err(e) = self
            .internal_handle_session_request(id, topic, request)
            .await
        {
            warn!("failed to get response from client session: '{e}'");
            self.send_response(u).await;
        }
    }
}
