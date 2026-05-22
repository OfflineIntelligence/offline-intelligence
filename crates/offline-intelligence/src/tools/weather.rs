//! Weather tool — Open-Meteo forecast via Nominatim geocoding.
//! No API key required. Rate limit: 1 req/s per IP (we make 2 sequential calls).

use anyhow::{bail, Result};
use serde_json::Value;
use super::Source;

const NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org/search";
const OPEN_METEO_URL: &str = "https://api.open-meteo.com/v1/forecast";

/// WMO weather code → human-readable description
fn wmo_description(code: i64) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Foggy",
        51 | 53 | 55 => "Drizzle",
        61 | 63 | 65 => "Rain",
        71 | 73 | 75 => "Snow",
        80 | 81 | 82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
}

pub async fn get_weather(
    client: &reqwest::Client,
    location: &str,
) -> Result<(Vec<Source>, String)> {
    // Step 1: geocode the location
    let geo: Value = client
        .get(NOMINATIM_URL)
        .query(&[("q", location), ("format", "json"), ("limit", "1")])
        .header("User-Agent", "OfflineIntelligence/0.1.4")
        .send()
        .await?
        .json()
        .await?;

    let places = geo.as_array().ok_or_else(|| anyhow::anyhow!("Nominatim returned non-array"))?;
    if places.is_empty() {
        bail!("Location '{}' not found", location);
    }

    let lat = places[0]["lat"].as_str().unwrap_or("0").parse::<f64>()?;
    let lon = places[0]["lon"].as_str().unwrap_or("0").parse::<f64>()?;
    let display_name = places[0]["display_name"]
        .as_str()
        .unwrap_or(location)
        .split(',')
        .take(2)
        .collect::<Vec<_>>()
        .join(",")
        .trim()
        .to_string();

    // Step 2: fetch weather
    let weather: Value = client
        .get(OPEN_METEO_URL)
        .query(&[
            ("latitude", lat.to_string().as_str()),
            ("longitude", lon.to_string().as_str()),
            ("current", "temperature_2m,apparent_temperature,weathercode,windspeed_10m,relativehumidity_2m"),
            ("daily", "temperature_2m_max,temperature_2m_min,weathercode,precipitation_sum"),
            ("timezone", "auto"),
            ("forecast_days", "3"),
        ])
        .send()
        .await?
        .json()
        .await?;

    let current = &weather["current"];
    let temp = current["temperature_2m"].as_f64().unwrap_or(0.0);
    let feels = current["apparent_temperature"].as_f64().unwrap_or(temp);
    let wind = current["windspeed_10m"].as_f64().unwrap_or(0.0);
    let humidity = current["relativehumidity_2m"].as_f64().unwrap_or(0.0);
    let wmo_code = current["weathercode"].as_i64().unwrap_or(0);
    let condition = wmo_description(wmo_code);

    // 3-day forecast
    let daily = &weather["daily"];
    let mut forecast_lines = Vec::new();
    if let (Some(times), Some(max_temps), Some(min_temps), Some(codes)) = (
        daily["time"].as_array(),
        daily["temperature_2m_max"].as_array(),
        daily["temperature_2m_min"].as_array(),
        daily["weathercode"].as_array(),
    ) {
        for i in 0..times.len().min(3) {
            let date = times[i].as_str().unwrap_or("?");
            let hi = max_temps[i].as_f64().unwrap_or(0.0);
            let lo = min_temps[i].as_f64().unwrap_or(0.0);
            let day_code = codes[i].as_i64().unwrap_or(0);
            forecast_lines.push(format!(
                "  {}: {}, High {:.0}°C / Low {:.0}°C",
                date,
                wmo_description(day_code),
                hi,
                lo
            ));
        }
    }

    let url = format!(
        "https://open-meteo.com/en/docs#latitude={}&longitude={}",
        lat, lon
    );

    let snippet = format!(
        "{}, Feels like {:.0}°C, Wind {:.0} km/h, Humidity {:.0}%",
        condition, feels, wind, humidity
    );

    let context = format!(
        "## Current Weather in {}\n\
         - Condition: {}\n\
         - Temperature: {:.1}°C (feels like {:.0}°C)\n\
         - Wind speed: {:.0} km/h\n\
         - Humidity: {:.0}%\n\
         \n\
         3-Day Forecast:\n{}\n",
        display_name,
        condition,
        temp, feels, wind, humidity,
        forecast_lines.join("\n")
    );

    let sources = vec![Source {
        id: 0, // will be renumbered by mod.rs
        title: format!("Weather in {}", display_name),
        url,
        snippet,
    }];

    Ok((sources, context))
}
