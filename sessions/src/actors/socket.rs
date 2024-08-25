use crate::{Atomic, PairingManager, SocketEvent};
use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use xtra::prelude::*;
use xtra::WeakAddress;

#[derive(Clone, Default, xtra::Actor)]
pub(crate) struct SocketActor {
    address: Option<Address<PairingManager>>,
}

impl Handler<SocketEvent> for SocketActor {
    type Return = ();

    async fn handle(&mut self, message: SocketEvent, ctx: &mut Context<Self>) -> Self::Return {
        if let Some(handler) = self.address.as_ref() {
            if let Err(e) = handler.send(message).await {
                warn!("failed to send socket event to handler");
            }
        } else {
            debug!("no socket handlers available");
        }
    }
}

impl Handler<Address<PairingManager>> for SocketActor {
    type Return = ();

    async fn handle(
        &mut self,
        message: Address<PairingManager>,
        ctx: &mut Context<Self>,
    ) -> Self::Return {
        self.address = Some(message);
    }
}
