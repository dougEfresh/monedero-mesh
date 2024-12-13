use {
    crate::Error,
    serde::{Deserialize, Serialize},
    solana_pubkey::Pubkey,
    solana_signature::Signature,
    std::fmt::{Display, Formatter},
};

#[derive(Serialize, Deserialize)]
pub struct WalletConnectTransaction {
    pub transaction: String,
}

#[derive(Serialize, Deserialize)]
pub struct SignMessageRequest {
    pub pubkey: Pubkey,
    pub message: String,
}

impl SignMessageRequest {
    pub fn new(pubkey: Pubkey, message: &str) -> Self {
        let message = bs58::encode(message).into_string();
        Self { pubkey, message }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaSignatureResponse {
    pub signature: String,
}

impl Display for SolanaSignatureResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.signature)
    }
}

impl From<&[u8]> for SolanaSignatureResponse {
    fn from(value: &[u8]) -> Self {
        let signature = bs58::encode(value).into_string();
        Self { signature }
    }
}

impl From<Signature> for SolanaSignatureResponse {
    fn from(value: Signature) -> Self {
        let signature = bs58::encode(value).into_string();
        Self { signature }
    }
}

impl TryFrom<SolanaSignatureResponse> for Signature {
    type Error = Error;

    fn try_from(value: SolanaSignatureResponse) -> std::result::Result<Self, Self::Error> {
        let decoded: Vec<u8> = bs58::decode(&value.signature)
            .into_vec()
            .map_err(|e| Error::Bs58Error(e.to_string()))?;
        let array: [u8; 64] = decoded
            .try_into()
            .map_err(|_| Error::SigError(value.signature))?;
        Ok(Self::from(array))
    }
}
