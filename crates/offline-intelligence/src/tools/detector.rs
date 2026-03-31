
use regex::Regex;
use super::ToolIntent;

lazy_static::lazy_static! {
    
    static ref WEATHER_RE: Regex = Regex::new(
        r"(?i)\b(?:weather|forecast|temperature|how(?:'s| is) it (?:in|at)|is it (?:raining|sunny|cold|hot|snowing)\s+in|current (?:weather|temp(?:erature)?))\b[^.?!]*?\bin\s+([\w\s,]+?)(?:\?|$|\.|\n|now|today|tomorrow|right now)"
    ).unwrap();

    static ref WEATHER_IN_RE: Regex = Regex::new(
        r"(?i)(?:weather|forecast|temperature)\s+(?:in|at|for)\s+([\w\s,]+?)(?:\?|$|\.|\n)"
    ).unwrap();

    static ref CURRENCY_AMOUNT_RE: Regex = Regex::new(
        r"(?i)\b(?:convert|exchange|how (?:much|many)|price of)?\s*(\d+(?:\.\d+)?)\s*([A-Za-z]{2,10})\s+(?:to|in|into)\s+([A-Za-z]{2,10})\b"
    ).unwrap();

    static ref CURRENCY_PAIR_RE: Regex = Regex::new(
        r"(?i)([A-Za-z]{2,10})\s+(?:to|in|into|vs\.?|against)\s+([A-Za-z]{2,10})"
    ).unwrap();

    static ref CRYPTO_RE: Regex = Regex::new(
        r"(?i)\b(?:price of|value of|how (?:much|many) (?:is|are))?\s*(bitcoin|btc|ethereum|eth|solana|sol|cardano|ada|dogecoin|doge|ripple|xrp|litecoin|ltc|bnb|matic|polygon|avalanche|avax|chainlink|link|polkadot|dot)\s+(?:price|value|cost|worth|in|to)?\s*(?:usd|eur|gbp|jpy|cad|aud)?\b"
    ).unwrap();

    static ref CRYPTO_PRICE_RE: Regex = Regex::new(
        r"(?i)\b(?:price|value|worth|cost)\s+(?:of\s+)?(bitcoin|btc|ethereum|eth|solana|sol|cardano|ada|dogecoin|doge|ripple|xrp|litecoin|ltc|bnb|matic|polygon|avalanche|avax|chainlink|link|polkadot|dot)\b"
    ).unwrap();
}

fn to_coingecko_id(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "bitcoin" | "btc" => "bitcoin",
        "ethereum" | "eth" => "ethereum",
        "solana" | "sol" => "solana",
        "cardano" | "ada" => "cardano",
        "dogecoin" | "doge" => "dogecoin",
        "ripple" | "xrp" => "ripple",
        "litecoin" | "ltc" => "litecoin",
        "bnb" => "binancecoin",
        "matic" | "polygon" => "matic-network",
        "avalanche" | "avax" => "avalanche-2",
        "chainlink" | "link" => "chainlink",
        "polkadot" | "dot" => "polkadot",
        _ => "bitcoin",
    }
}

fn is_crypto_name(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "bitcoin" | "btc" | "ethereum" | "eth" | "solana" | "sol"
            | "cardano" | "ada" | "dogecoin" | "doge" | "ripple" | "xrp"
            | "litecoin" | "ltc" | "bnb" | "matic" | "polygon"
            | "avalanche" | "avax" | "chainlink" | "link" | "polkadot" | "dot"
    )
}

fn normalize_currency(s: &str) -> Option<String> {
    let code: &str = match s.to_lowercase().as_str() {
        "usd" | "dollar" | "dollars" => "USD",
        "eur" | "euro" | "euros" => "EUR",
        "gbp" | "pound" | "pounds" | "sterling" => "GBP",
        "jpy" | "yen" => "JPY",
        "cad" | "canadian" => "CAD",
        "aud" | "australian" => "AUD",
        "chf" | "franc" => "CHF",
        "inr" | "rupee" | "rupees" => "INR",
        "cny" | "yuan" | "rmb" | "renminbi" => "CNY",
        "brl" | "real" | "reais" => "BRL",
        "mxn" | "peso" | "pesos" => "MXN",
        "sgd" => "SGD",
        "hkd" => "HKD",
        "nok" | "krone" => "NOK",
        "sek" | "krona" => "SEK",
        "dkk" => "DKK",
        "nzd" => "NZD",
        "zar" | "rand" => "ZAR",
        "krw" | "won" => "KRW",
        "try" | "lira" => "TRY",
        "aed" | "dirham" => "AED",
        "thb" | "baht" => "THB",
        "php" => "PHP",
        "idr" | "rupiah" => "IDR",
        "myr" | "ringgit" => "MYR",
        other if other.len() == 3 && other.chars().all(|c| c.is_ascii_alphabetic()) => {
            return Some(other.to_uppercase());
        }
        _ => return None,
    };
    Some(code.to_string())
}

pub fn detect_intents(user_message: &str) -> Vec<ToolIntent> {
    let mut intents: Vec<ToolIntent> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let location = WEATHER_IN_RE
        .captures(user_message)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .or_else(|| {
            WEATHER_RE
                .captures(user_message)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
        });

    if let Some(loc) = location {
        if !loc.is_empty() && !seen.contains(&format!("weather:{}", loc)) {
            seen.insert(format!("weather:{}", loc));
            intents.push(ToolIntent::Weather(loc));
        }
    }

    if let Some(caps) = CURRENCY_AMOUNT_RE.captures(user_message) {
        let amount: f64 = caps[1].parse().unwrap_or(1.0);
        let from_raw = &caps[2];
        let to_raw = &caps[3];
        if !is_crypto_name(from_raw) && !is_crypto_name(to_raw) {
            if let (Some(from), Some(to)) =
                (normalize_currency(from_raw), normalize_currency(to_raw))
            {
                let key = format!("currency:{}:{}", from, to);
                if !seen.contains(&key) {
                    seen.insert(key);
                    intents.push(ToolIntent::Currency { from, to, amount });
                }
            }
        }
    }

    if !intents.iter().any(|i| matches!(i, ToolIntent::Currency { .. })) {
        for caps in CURRENCY_PAIR_RE.captures_iter(user_message) {
            let from_raw = &caps[1];
            let to_raw = &caps[2];

            if is_crypto_name(from_raw) || is_crypto_name(to_raw) {
                continue;
            }

            if let (Some(from), Some(to)) =
                (normalize_currency(from_raw), normalize_currency(to_raw))
            {
                if from == to {
                    continue;
                }
                let key = format!("currency:{}:{}", from, to);
                if !seen.contains(&key) {
                    seen.insert(key);
                    intents.push(ToolIntent::Currency { from, to, amount: 1.0 });
                    break;
                }
            }
        }
    }

    let crypto_match = CRYPTO_PRICE_RE
        .captures(user_message)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_lowercase())
        .or_else(|| {
            CRYPTO_RE
                .captures(user_message)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_lowercase())
        });

    if let Some(coin_raw) = crypto_match {
        let coin_id = to_coingecko_id(&coin_raw).to_string();
        let key = format!("crypto:{}", coin_id);
        if !seen.contains(&key) {
            seen.insert(key);
            intents.push(ToolIntent::Crypto {
                coin: coin_id,
                vs: "usd".to_string(),
            });
        }
    }

    intents.truncate(3);
    intents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_detection() {
        let intents = detect_intents("What is the weather in London today?");
        assert!(intents.iter().any(|i| matches!(i, ToolIntent::Weather(l) if l.contains("London"))));
    }

    #[test]
    fn test_currency_with_amount() {
        let intents = detect_intents("Convert 100 USD to EUR please");
        assert!(intents.iter().any(|i| matches!(i, ToolIntent::Currency { .. })));
    }

    #[test]
    fn test_currency_pair_no_amount() {
        let intents = detect_intents("how much is USD to INR today?");
        assert!(intents.iter().any(|i| matches!(i, ToolIntent::Currency { from, to, .. } if from == "USD" && to == "INR")));
    }

    #[test]
    fn test_currency_pair_bare() {
        let intents = detect_intents("USD to INR");
        assert!(intents.iter().any(|i| matches!(i, ToolIntent::Currency { .. })));
    }

    #[test]
    fn test_crypto_detection() {
        let intents = detect_intents("What is the price of bitcoin?");
        assert!(intents.iter().any(|i| matches!(i, ToolIntent::Crypto { .. })));
    }

    #[test]
    fn test_no_false_positive_currency() {
        let intents = detect_intents("I want to go to school today");
        assert!(!intents.iter().any(|i| matches!(i, ToolIntent::Currency { .. })));
    }

    #[test]
    fn test_general_questions_no_tools() {
        
        let intents = detect_intents("What is quantum computing?");
        assert!(intents.is_empty());
        let intents = detect_intents("who is the current president of USA?");
        assert!(intents.is_empty());
    }
}
