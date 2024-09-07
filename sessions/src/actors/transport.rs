use std::fmt::{Debug, Formatter};
use crate::actors::{AddRequest, ClearPairing, InboundResponseActor, SendRequest, Unsubscribe};
use crate::domain::{MessageId, SubscriptionId};
use crate::rpc::{
    IrnMetadata, RelayProtocolMetadata, Request, Response, RpcResponse, RpcResponsePayload,
};
use crate::Cipher;
use crate::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{debug, error, warn};
use walletconnect_relay::Client;
use xtra::{Address, Context, Handler};

#[derive(Clone, xtra::Actor)]
pub struct TransportActor {
    cipher: Cipher,
    relay: Option<Client>,
    inbound_response_actor: Address<InboundResponseActor>,
}

impl Debug for TransportActor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[actor-transport]")
    }
}

impl Handler<ClearPairing> for TransportActor {
    type Return = ();

    async fn handle(&mut self, _message: ClearPairing, _ctx: &mut Context<Self>) -> Self::Return {
        // TODO: Do I unsubscribe?
        self.cipher.reset();
        if let Err(e) = self.inbound_response_actor.send(ClearPairing).await {
            warn!("failed to clean inbound responder: {e}");
        }
    }
}

async fn send_response(result: RpcResponse, cipher: Cipher, relay: Client) {
    let irn_metadata: IrnMetadata = match &result.payload {
        RpcResponsePayload::Success(s) => s.irn_metadata(),
        RpcResponsePayload::Error(e) => e.irn_metadata(),
    };

    let response: Response = match result.payload {
        RpcResponsePayload::Success(s) => {
            let params = s.try_into();
            if let Err(e) = params {
                warn!("failed to deserialize response {e}");
                return;
            }
            Response::new(result.id, params.unwrap())
        }
        RpcResponsePayload::Error(e) => {
            let params = e.try_into();
            if let Err(e) = params {
                warn!("failed to deserialize error response {e}");
                return;
            }
            Response::new(result.id, params.unwrap())
        }
    };

    match cipher.encode(&result.topic, &response) {
        Ok(encrypted) => {
            if let Err(e) = relay
                .publish(
                    result.topic.clone(),
                    Arc::from(encrypted),
                    irn_metadata.tag,
                    Duration::from_secs(irn_metadata.ttl),
                    irn_metadata.prompt,
                )
                .await
            {
                error!(
                    "failed to publish payload  error: '{e}' on topic {}",
                    result.topic
                );
            }
        }
        Err(err) => {
            error!("failed to encrypt payload {err}");
            debug!("failed encrypting {:#?}", response);
        }
    };
}

impl TransportActor {
    pub(crate) fn new(
        cipher: Cipher,
        inbound_response_actor: Address<InboundResponseActor>,
    ) -> Self {
        Self {
            cipher,
            inbound_response_actor,
            relay: None,
        }
    }
}

impl Handler<Unsubscribe> for TransportActor {
    type Return = Result<()>;

    async fn handle(&mut self, message: Unsubscribe, _ctx: &mut Context<Self>) -> Self::Return {
        let relay = self.relay.as_ref().ok_or(crate::Error::NoClient)?;
        Ok(relay.unsubscribe(message.0).await?)
    }
}

impl Handler<Client> for TransportActor {
    type Return = ();

    async fn handle(&mut self, message: Client, _ctx: &mut Context<Self>) -> Self::Return {
        self.relay = Some(message);
    }
}

impl Handler<RpcResponse> for TransportActor {
    type Return = Result<()>;

    async fn handle(&mut self, message: RpcResponse, _ctx: &mut Context<Self>) -> Self::Return {
        let relay = self.relay.clone().ok_or(crate::Error::NoClient)?;
        let cipher = self.cipher.clone();
        tokio::spawn(async move {
            send_response(message, cipher, relay).await;
        });
        Ok(())
    }
}

impl Handler<SendRequest> for TransportActor {
    type Return = Result<(MessageId, Duration, oneshot::Receiver<Response>)>;

    #[tracing::instrument(skip(_ctx), level = "info", fields(message = message.to_string()))]
    async fn handle(&mut self, message: SendRequest, _ctx: &mut Context<Self>) -> Self::Return {
        let relay = self.relay.as_ref().ok_or(crate::Error::NoClient)?;
        let (id, rx) = self.inbound_response_actor.send(AddRequest).await?;

        let topic = message.0;
        let params = message.1;
        let irn_metadata = params.irn_metadata();
        let request = Request::new(id, params);
        let encrypted = self.cipher.encode(&topic, &request)?;
        let ttl = Duration::from_secs(irn_metadata.ttl);
        relay
            .publish(
                topic,
                Arc::from(encrypted),
                irn_metadata.tag,
                ttl,
                irn_metadata.prompt,
            )
            .await?;
        Ok((id, ttl, rx))
    }
}

/*
#[cfg(feature = "mock")]
#[cfg(test)]
mod test {
    use super::*;
    use crate::actors::InboundResponseActor;
    use crate::crypto::CipherError;
    use crate::relay::mock::test::DummyHandler;
    use crate::rpc::{PairPingRequest, RequestParams};
    use crate::{KvStorage, Pairing};
    use xtra::Mailbox;

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_send() -> anyhow::Result<()> {
        crate::test::init_tracing();
        let inbound = xtra::spawn_tokio(InboundResponseActor::default(), Mailbox::unbounded());
        let cipher: Cipher = Cipher::new(Arc::new(KvStorage::default()), None)?;
        let transport = TransportActor::new(cipher.clone(), inbound);
        let actor = xtra::spawn_tokio(transport.clone(), Mailbox::unbounded());
        let pairing = Pairing::default();
        let topic = pairing.topic.clone();
        let params = RequestParams::PairPing(PairPingRequest {});
        let result = actor
            .send(SendRequest(topic.clone(), params.clone()))
            .await?;
        assert!(matches!(result, Err(crate::Error::NoClient)));
        let handler = DummyHandler::new();
        let client = Client::mock(handler.clone());
        actor.send(client.clone()).await?;
        let result = actor
            .send(SendRequest(topic.clone(), params.clone()))
            .await?;
        assert!(matches!(
            result,
            Err(crate::Error::CipherError(CipherError::UnknownTopic(_)))
        ));
        cipher.set_pairing(Some(pairing))?;
        let result = actor
            .send(SendRequest(topic.clone(), params.clone()))
            .await?;

        assert!(matches!(
            result,
            Err(crate::Error::ConnectError(
                crate::relay::ClientError::Disconnected
            ))
        ));
        Ok(())
    }
}

 */
