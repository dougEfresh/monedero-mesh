use crate::actors::ClearPairing;
use crate::domain::Topic;
use crate::rpc::{Proposer, SessionProposeResponse};
use crate::Result;
use crate::{Cipher, Pairing};
use std::future::Future;
use xtra::prelude::*;

#[derive(Clone, xtra::Actor)]
pub struct CipherActor {
    cipher: Cipher,
}

impl CipherActor {
    pub fn new(cipher: Cipher) -> Self {
        Self { cipher }
    }
}

impl Handler<Proposer> for CipherActor {
    type Return = Result<Topic>;

    async fn handle(&mut self, message: Proposer, _ctx: &mut Context<Self>) -> Self::Return {
        let (topic, _) = self.cipher.create_common_topic(message.public_key)?;
        Ok(topic)
    }
}

impl Handler<Pairing> for CipherActor {
    type Return = Result<()>;

    async fn handle(&mut self, message: Pairing, _ctx: &mut Context<Self>) -> Self::Return {
        self.cipher.set_pairing(Some(message))?;
        Ok(())
    }
}

impl Handler<ClearPairing> for CipherActor {
    type Return = Result<()>;

    async fn handle(&mut self, _message: ClearPairing, _ctx: &mut Context<Self>) -> Self::Return {
        self.cipher.set_pairing(None)?;
        Ok(())
    }
}

impl Handler<SessionProposeResponse> for CipherActor {
    type Return = Result<Topic>;

    async fn handle(
        &mut self,
        message: SessionProposeResponse,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        let (topic, _) = self
            .cipher
            .create_common_topic(message.responder_public_key)?;
        Ok(topic)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::KvStorage;
    use std::sync::Arc;

    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn test_cipher_actor() -> anyhow::Result<()> {
        let cipher = Cipher::new(Arc::new(KvStorage::default()), None)?;
        let actor = xtra::spawn_tokio(CipherActor::new(cipher.clone()), Mailbox::unbounded());
        let pair = Pairing::default();

        let _ = actor.send(pair.clone()).await?;
        assert!(cipher.pairing().is_some());

        let _ = actor.send(ClearPairing).await?;
        assert!(cipher.pairing().is_none());
        let _ = actor.send(pair.clone()).await?;

        let wallet = Cipher::new(Arc::new(KvStorage::default()), None)?;
        wallet.set_pairing(Some(pair.clone()))?;
        let (common, _) = wallet.create_common_topic(
            cipher
                .public_key_hex()
                .ok_or(crate::Error::NoPairingTopic)?,
        )?;
        let settlement_resp = SessionProposeResponse {
            relay: Default::default(),
            responder_public_key: wallet
                .public_key_hex()
                .ok_or(crate::Error::NoPairingTopic)?,
        };
        let topic = actor.send(settlement_resp).await??;

        assert_eq!(topic, common);
        Ok(())
    }
}
