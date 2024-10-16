use std::fmt::{Debug, Display, Formatter};
use solana_sdk::pubkey::Pubkey;

#[derive( Copy, Clone, Eq, PartialEq, Hash)]
pub enum AccountType {
    Native(Pubkey),
    Token(Pubkey),
    Token2022(Pubkey),
    Stake(Pubkey),
}

impl AccountType {
    fn fmt_common(&self) -> String {
        match self {
            AccountType::Native(k) => format!( "native:{}", k),
            AccountType::Token(k) => format!( "token:{}", k),
            AccountType::Token2022(k) => format!( "token2022:{}", k),
            AccountType::Stake(k) => format!("stake:{}", k)
        }
    }
}

impl AccountType {
    pub fn pubkey(&self) -> &Pubkey {
        match self {
            AccountType::Native(k)  | AccountType::Stake(k) | AccountType::Token(k) | AccountType::Token2022(k) => k,
        }
    }
}

impl Debug for AccountType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_common())
    }
}

impl Display for AccountType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_common())
    }
}
