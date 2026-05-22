//! Web tools module — intent-driven external API integrations.
//!
//! Detects user intent from the latest message and calls the appropriate
//! free public APIs (no keys required). Results are injected as a system
//! context block before the LLM receives the conversation.
//!
//! Supported tools:
//!   - Weather  — Open-Meteo + Nominatim geocoding (keyless)
//!   - Currency — ExchangeRate-API fiat (keyless, 160+ currencies)
//!   - Crypto   — CoinGecko free API (keyless)
//!
//! Timeout policy (applies to all models and modes — online/offline, Windows/macOS):
//!   - Each individual tool: 8 seconds (per-tool cap)
//!   - Entire run_tools() call: 10 seconds (hard deadline)
//!   - If all tools time out / fail: returns None → LLM answers from training data

pub mod detector;
pub mod weather;
pub mod currency;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Timeout for a single tool's HTTP calls (e.g. one weather request).
const TOOL_TIMEOUT_SECS: u64 = 8;

/// Hard deadline for the entire run_tools() execution (all tools combined).
/// Tools run in parallel so this is effectively the slowest tool's cap.
const TOTAL_TIMEOUT_SECS: u64 = 10;

/// A single cited source shown in the frontend citation panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: usize,
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Aggregated result from one or more tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResults {
    /// Numbered citation sources for the frontend panel.
    pub sources: Vec<Source>,
    /// Concatenated context text to prepend as a system message for the LLM.
    pub context: String,
}

/// What tool(s) the detector thinks should run for this user message.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolIntent {
    Weather(String),     // location string
    Currency { from: String, to: String, amount: f64 },
    Crypto { coin: String, vs: String },
}

/// Run all detected tools in parallel and aggregate results.
///
/// Returns `None` when:
///   - no intents were detected for the message
///   - all tool calls failed or timed out (graceful offline fallback)
///
/// Enforces a hard 10-second deadline so slow public APIs never stall the LLM.
pub async fn run_tools(
    user_message: &str,
    http_client: &reqwest::Client,
) -> Option<ToolResults> {
    let intents = detector::detect_intents(user_message);
    if intents.is_empty() {
        return None;
    }

    info!(
        "Web tools triggered: {:?}",
        intents.iter().map(|i| format!("{:?}", i)).collect::<Vec<_>>()
    );

    // Apply the hard total deadline around the entire fetch + aggregate phase.
    match tokio::time::timeout(
        std::time::Duration::from_secs(TOTAL_TIMEOUT_SECS),
        run_tools_inner(http_client, intents),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            warn!(
                "Web tools hard deadline ({}s) exceeded — skipping tool context",
                TOTAL_TIMEOUT_SECS
            );
            None
        }
    }
}

/// Inner implementation (wrapped by the hard deadline in `run_tools`).
async fn run_tools_inner(
    http_client: &reqwest::Client,
    intents: Vec<ToolIntent>,
) -> Option<ToolResults> {
    // Spawn all tools concurrently, each with its own per-tool timeout.
    let mut handles: Vec<tokio::task::JoinHandle<Option<(Vec<Source>, String)>>> = Vec::new();

    for intent in intents {
        let client = http_client.clone();

        handles.push(tokio::spawn(async move {
            // Per-tool timeout: if a single public API hangs, we give up cleanly.
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(TOOL_TIMEOUT_SECS),
                async {
                    match &intent {
                        ToolIntent::Weather(loc) => {
                            weather::get_weather(&client, loc)
                                .await
                                .map_err(|e| { warn!("Weather tool error: {}", e); e })
                                .ok()
                        }
                        ToolIntent::Currency { from, to, amount } => {
                            currency::convert_fiat(&client, from, to, *amount)
                                .await
                                .map_err(|e| { warn!("Currency tool error: {}", e); e })
                                .ok()
                        }
                        ToolIntent::Crypto { coin, vs } => {
                            currency::get_crypto_price(&client, coin, vs)
                                .await
                                .map_err(|e| { warn!("Crypto tool error: {}", e); e })
                                .ok()
                        }
                    }
                },
            )
            .await;

            match result {
                Ok(inner) => inner,
                Err(_) => {
                    warn!(
                        "Tool {:?} timed out after {}s",
                        std::mem::discriminant(&intent),
                        TOOL_TIMEOUT_SECS
                    );
                    None
                }
            }
        }));
    }

    // Collect all results (handles run in parallel; awaiting them is just joining).
    let mut all_sources: Vec<Source> = Vec::new();
    let mut context_parts: Vec<String> = Vec::new();
    let mut id_counter = 1usize;

    for handle in handles {
        if let Ok(Some((mut sources, ctx))) = handle.await {
            for s in &mut sources {
                s.id = id_counter;
                id_counter += 1;
            }
            all_sources.extend(sources);
            context_parts.push(ctx);
        }
    }

    if all_sources.is_empty() && context_parts.is_empty() {
        return None;
    }

    // Build numbered citation references for the context
    let citation_map: String = all_sources
        .iter()
        .map(|s| format!("[{}] {} — {}", s.id, s.title, s.url))
        .collect::<Vec<_>>()
        .join("\n");

    // Compute full calendar date from Unix timestamp — no external crates needed.
    let today_str = {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut z = (secs / 86400) as i64 + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        z -= era * 146097;
        let doe = z as u64;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        format!("{}-{:02}-{:02}", y, m, d)
    };

    let mut context = format!(
        "=== LIVE DATA (fetched on {}) ===\n\
         IMPORTANT: Today's date is {}. The following is real-time information retrieved\n\
         from live data sources right now. Use this data — do NOT answer from training\n\
         knowledge for these facts. Use [N] notation to cite sources.\n\n",
        today_str, today_str
    );
    for part in &context_parts {
        context.push_str(part);
        context.push('\n');
    }
    if !citation_map.is_empty() {
        context.push_str("\nSources:\n");
        context.push_str(&citation_map);
        context.push('\n');
    }
    context.push_str("=== End of live data ===\n");

    Some(ToolResults {
        sources: all_sources,
        context,
    })
}
