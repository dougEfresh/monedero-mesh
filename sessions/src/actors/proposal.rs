use {
    crate::{
        actors::TransportActor,
        rpc::{
            ErrorParams,
            IntoUnknownError,
            RequestParams,
            ResponseParamsError,
            RpcRequest,
            RpcResponse,
        },
        Dapp,
        Wallet,
    },
    monedero_domain::SessionSettled,
    tracing::{error, info, warn},
    xtra::{prelude::*, Address},
};

#[derive(Clone, Actor)]
pub struct ProposalActor {
    dapp: Option<Address<Dapp>>,
    wallet: Option<Address<Wallet>>,
    pub(super) responder: Address<TransportActor>,
}

impl Handler<Dapp> for ProposalActor {
    type Return = ();

    async fn handle(&mut self, message: Dapp, _ctx: &mut Context<Self>) -> Self::Return {
        let addr = xtra::spawn_tokio(message, Mailbox::unbounded());
        self.dapp = Some(addr)
    }
}

impl Handler<Wallet> for ProposalActor {
    type Return = ();

    async fn handle(&mut self, message: Wallet, _ctx: &mut Context<Self>) -> Self::Return {
        let addr = xtra::spawn_tokio(message, Mailbox::unbounded());
        self.wallet = Some(addr)
    }
}

impl ProposalActor {
    pub fn new(responder: Address<TransportActor>) -> Self {
        Self {
            dapp: None,
            wallet: None,
            responder,
        }
    }

    pub(super) async fn send_response(&self, resp: RpcResponse) {
        let id = resp.id.clone();
        let topic = resp.topic.clone();
        if let Err(err) = self.responder.send(resp).await {
            warn!(
                "Failed to send response for id {} on topic {} {}",
                id, topic, err
            );
        }
    }
}

impl Handler<RpcRequest> for ProposalActor {
    type Return = ();

    async fn handle(&mut self, message: RpcRequest, _ctx: &mut Context<Self>) -> Self::Return {
        let id = message.payload.id;
        let topic = message.topic.clone();
        let response: RpcResponse = match message.payload.params {
            RequestParams::SessionPropose(args) => {
                info!("got session proposal");
                let unknown = RpcResponse::unknown(
                    id,
                    topic.clone(),
                    ResponseParamsError::SessionPropose(ErrorParams::unknown()),
                );
                match &self.wallet {
                    None => {
                        error!("no wallet found for proposal");
                        unknown
                    }
                    Some(wallet) => wallet
                        .send(args)
                        .await
                        .map(|payload| RpcResponse { id, topic, payload })
                        .unwrap_or(unknown),
                }
            }
            RequestParams::SessionSettle(args) => {
                let unknown = RpcResponse::unknown(id, topic.clone(), (&args).unknown());
                match &self.dapp {
                    None => {
                        error!("no dapp found for settlement");
                        unknown
                    }
                    Some(dapp) => dapp
                        .send(SessionSettled {
                            topic: topic.clone(),
                            namespaces: args.namespaces,
                            expiry: args.expiry,
                        })
                        .await
                        .map(|payload| RpcResponse { id, topic, payload })
                        .unwrap_or(unknown),
                }
            }
            _ => {
                warn!("got non proposal request {:#?}", message);
                return;
            }
        };
        self.send_response(response).await;
    }
}
