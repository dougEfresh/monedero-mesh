use crate::method::Method;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

const PERSONAL_SIGN: &str = "personal_sign";
const SIGN_TRANSACTION: &str = "eth_signTransaction";
const SIGN: &str = "eth_sign";
const PERSONAL_SIGN_EXT: &str = "personal_signExt";
const SIGN_TYPED_DATA: &str = "eth_signTypedData";
const SIGN_TYPED_DAVA_V4: &str = "eth_signTypedData_v4";
const SEND_TRANSACTION: &str = "eth_sendTransaction";
const SEND_TRANSACTION_EXT: &str = "eth_sendTransactionExt";

#[derive(Debug, Clone, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub enum EipMethod {
    SignTransaction,
    Sign,
    PersonalSign,
    PersonalSignExt,
    SignTypedData,
    SignTypedDataV4,
    SendTransaction,
    SendTransactionExt,
    Other(String),
}

impl Ord for EipMethod {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl PartialOrd for EipMethod {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for EipMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SignTransaction => write!(f, "{SIGN_TRANSACTION}"),
            Self::Sign => write!(f, "{SIGN}"),
            Self::PersonalSign => write!(f, "{PERSONAL_SIGN}"),
            Self::PersonalSignExt => write!(f, "{PERSONAL_SIGN_EXT}"),
            Self::SignTypedData => write!(f, "{SIGN_TYPED_DATA}"),
            Self::SignTypedDataV4 => write!(f, "{SIGN_TYPED_DAVA_V4}"),
            Self::SendTransaction => write!(f, "{SEND_TRANSACTION}"),
            Self::SendTransactionExt => write!(f, "{SEND_TRANSACTION_EXT}"),
            Self::Other(m) => write!(f, "{m}"),
        }
    }
}

impl EipMethod {
    #[must_use]
    pub fn defaults() -> BTreeSet<Method> {
        BTreeSet::from([
            Method::EIP155(Self::PersonalSign),
            Method::EIP155(Self::SendTransaction),
            Method::EIP155(Self::SignTransaction),
            Method::EIP155(Self::SignTypedDataV4),
            Method::EIP155(Self::SignTypedData),
            Method::EIP155(Self::Sign),
        ])
    }
}

impl FromStr for EipMethod {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            PERSONAL_SIGN => Ok(Self::PersonalSign),
            SIGN_TRANSACTION => Ok(Self::SignTransaction),
            SIGN => Ok(Self::Sign),
            PERSONAL_SIGN_EXT => Ok(Self::PersonalSignExt),
            SIGN_TYPED_DATA => Ok(Self::SignTypedData),
            SIGN_TYPED_DAVA_V4 => Ok(Self::SignTypedDataV4),
            SEND_TRANSACTION => Ok(Self::SendTransaction),
            SEND_TRANSACTION_EXT => Ok(Self::SendTransactionExt),
            _ => Ok(Self::Other(String::from(s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eip_method() -> anyhow::Result<()> {
        assert_eq!(
            EipMethod::PersonalSignExt,
            PERSONAL_SIGN_EXT.parse::<EipMethod>()?
        );
        assert_eq!(EipMethod::PersonalSign, PERSONAL_SIGN.parse::<EipMethod>()?);
        assert_eq!(
            EipMethod::SendTransaction,
            SEND_TRANSACTION.parse::<EipMethod>()?
        );
        assert_eq!(
            EipMethod::SendTransactionExt,
            SEND_TRANSACTION_EXT.parse::<EipMethod>()?
        );
        assert!(matches!(
            "eth_signAndSend".parse::<EipMethod>()?,
            EipMethod::Other(_)
        ));
        Ok(())
    }
}
