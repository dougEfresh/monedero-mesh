use {
    anyhow::Result,
    serde::{Deserialize, Deserializer},
    solana_sdk::bs58,
    std::{collections::HashSet, fs::File, io::BufReader},
};

#[derive(Clone, Deserialize)]
#[allow(dead_code)]
struct Token {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub mint_authority: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_null_default")]
    pub daily_volume: f64,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

fn clean_symbol(s: &str) -> String {
    s.replace('$', "").to_uppercase()
}

fn generate_token_code(tokens: Vec<Token>, dev: bool) -> anyhow::Result<()> {
    let mut symbols = HashSet::new();
    let suffix = if dev { "Dev" } else { "" };
    println!();
    println!("pub enum TokenSymbol{} {{", suffix);

    for token in &tokens {
        if !symbols.contains(&token.symbol) {
            println!("    {},", clean_symbol(&token.symbol));
            symbols.insert(&token.symbol);
        }
    }

    println!("    Other(Pubkey),");
    println!("}}");
    println!();

    for token in &tokens {
        let decoded = bs58::decode(&token.address).into_vec()?;
        println!("// Token: {} ({})", token.name, token.symbol);
        if dev {
            println!(
                "pub const {}_ADDRESS_{}: Pubkey = Pubkey::new_from_array([",
                clean_symbol(&token.symbol),
                suffix.to_uppercase()
            );
        } else {
            println!(
                "pub const {}_ADDRESS: Pubkey = Pubkey::new_from_array([",
                clean_symbol(&token.symbol)
            );
        }
        for (i, byte) in decoded.iter().enumerate() {
            if i % 8 == 0 {
                print!("    ");
            }
            print!("{:#04x}, ", byte);
            if i % 8 == 7 {
                println!();
            }
        }
        if decoded.len() % 8 != 0 {
            println!();
        }
        println!("]);");
        println!();
    }
    println!("impl From<TokenSymbol{}> for Pubkey {{", suffix);
    println!("fn from(value: TokenSymbol{}) -> Self {{", suffix);
    println!("match value {{");
    for token in &tokens {
        let symbol = clean_symbol(&token.symbol);
        if dev {
            println!(
                "      TokenSymbol{}::{} => {}_ADDRESS_{},",
                suffix,
                symbol,
                symbol,
                suffix.to_uppercase()
            );
        } else {
            println!(
                "      TokenSymbol{}::{} => {}_ADDRESS,",
                suffix, symbol, symbol
            );
        }
    }
    println!("      TokenSymbol{}::Other(pk) => pk", suffix);
    println!("      {}\n    {}\n {}", "}", "}", "}");

    Ok(())
}

fn main() -> Result<()> {
    // Read tokens from a local file
    let file = File::open("solana/tokens.json")?;
    let reader = BufReader::new(file);
    let mut tokens: Vec<Token> = serde_json::from_reader(reader)?;
    // Commented out HTTP request code
    // let url = "https://tokens.jup.ag/tokens?tags=verified";
    // let response = reqwest::blocking::get(url)?.text()?;
    // let tokens: Vec<Token> = serde_json::from_str(&response)?;

    // Sort tokens by daily volume (descending order)
    tokens.sort_by(|a, b| b.daily_volume.partial_cmp(&a.daily_volume).unwrap());
    let mut keep = tokens
        .iter()
        .cloned()
        .into_iter()
        .filter(|t| t.symbol.contains("ORE"))
        .collect();
    let mut top_tokens: Vec<Token> = tokens.into_iter().take(20).collect();
    // Now sort these top 20 tokens alphabetically by symbol
    top_tokens.append(&mut keep);
    top_tokens.sort_by_key(|s| clean_symbol(s.symbol.as_str()));
    println!("use solana_program::pubkey::Pubkey;");
    generate_token_code(top_tokens, false)?;
    // devnet
    let dev = vec![
        Token {
            address: "So11111111111111111111111111111111111111112".to_string(),
            name: "WSOL".to_string(),
            symbol: "SOL".to_string(),
            decimals: 0,
            mint_authority: None,
            daily_volume: 0.0,
        },
        Token {
            address: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
            name: "DEV USDC".to_string(),
            symbol: "USDC".to_string(),
            decimals: 0,
            mint_authority: None,
            daily_volume: 0.0,
        },
    ];
    generate_token_code(dev, true)?;
    Ok(())
}
