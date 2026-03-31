
use anyhow::Result;
use serde_json::Value;
use super::Source;

const EXCHANGERATE_URL: &str = "https://api.exchangerate-api.com/v4/latest";
const COINGECKO_URL: &str = "https://api.coingecko.com/api/v3/simple/price";

pub async fn convert_fiat(
    client: &reqwest::Client,
    from: &str,
    to: &str,
    amount: f64,
) -> Result<(Vec<Source>, String)> {
    let from_upper = from.to_uppercase();
    let to_upper = to.to_uppercase();

    let url = format!("{}/{}", EXCHANGERATE_URL, from_upper);
    let resp: Value = client
        .get(&url)
        .send()
        .await?
        .json()
        .await?;

    let rate = resp["rates"][&to_upper]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("Currency pair {}/{} not found in ExchangeRate-API response", from_upper, to_upper))?;

    let date = resp["date"].as_str().unwrap_or("today").to_string();
    let converted = amount * rate;

    let context = format!(
        "## Currency Conversion\n\
         {:.2} {} = {:.4} {} (rate as of {}, source: ExchangeRate-API)\n",
        amount, from_upper, converted, to_upper, date
    );

    let snippet = format!("{:.2} {} = {:.4} {} ({})", amount, from_upper, converted, to_upper, date);

    let sources = vec![Source {
        id: 0,
        title: format!("{} to {} Exchange Rate", from_upper, to_upper),
        url: "https://www.exchangerate-api.com".to_string(),
        snippet,
    }];

    Ok((sources, context))
}

pub async fn get_crypto_price(
    client: &reqwest::Client,
    coin_id: &str,   
    vs: &str,        
) -> Result<(Vec<Source>, String)> {
    let resp: Value = client
        .get(COINGECKO_URL)
        .query(&[
            ("ids", coin_id),
            ("vs_currencies", vs),
            ("include_24hr_change", "true"),
            ("include_market_cap", "true"),
        ])
        .send()
        .await?
        .json()
        .await?;

    let coin_data = resp[coin_id]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Coin '{}' not found in CoinGecko response", coin_id))?;

    let price = coin_data[vs].as_f64().unwrap_or(0.0);
    let change_24h = coin_data[&format!("{}_24h_change", vs)].as_f64().unwrap_or(0.0);
    let market_cap = coin_data[&format!("{}_market_cap", vs)].as_f64().unwrap_or(0.0);

    let change_sign = if change_24h >= 0.0 { "+" } else { "" };
    let coin_name = capitalize(coin_id);

    let context = format!(
        "## {} Price ({} uppercase)\n\
         - Price: {:.2} {} \n\
         - 24h Change: {}{:.2}%\n\
         - Market Cap: {:.0} {}\n",
        coin_name,
        vs.to_uppercase(),
        price, vs.to_uppercase(),
        change_sign, change_24h,
        market_cap, vs.to_uppercase()
    );

    let snippet = format!(
        "{}: {:.2} {} ({}{:.2}% 24h)",
        coin_name, price, vs.to_uppercase(), change_sign, change_24h
    );

    let sources = vec![Source {
        id: 0,
        title: format!("{} Price — CoinGecko", coin_name),
        url: format!("https://www.coingecko.com/en/coins/{}", coin_id),
        snippet,
    }];

    Ok((sources, context))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
