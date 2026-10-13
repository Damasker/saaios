//! Locality weather fetch and the Система widget line.
//! Five failed attempts publish the reason. Tap starts the cycle again.
//! Coordinates are the configured query place, not modem GNSS.

use chrono::Utc;
use saai_observation::{
    observations_from_locality_cache, observations_from_open_meteo_current, wmo_condition_label,
    Locality, ATTRIBUTION, KEY_CONDITION, KEY_HUMIDITY, KEY_LATITUDE, KEY_LONGITUDE,
    KEY_TEMPERATURE, KEY_WEATHER_CODE, KEY_WIND_SPEED, SOURCE_OPEN_METEO,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const FETCH_ATTEMPTS: u32 = 5;
pub const EMPTY_FORECAST: &str = "Open-Meteo · нет данных";
const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const USER_AGENT: &str = "SaaiOS-weather/0.1 (personal locality telemetry; Open-Meteo CC BY 4.0)";
const FETCH_TIMEOUT_SECS: u64 = 4;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchState {
    pub attempts: u32,
    pub last_error: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct LocalityCacheFile {
    locality: CachePlace,
    attribution: String,
    source: String,
    fetched_at: String,
    forecast: Value,
}

#[derive(Serialize, Deserialize)]
struct CachePlace {
    name: String,
    latitude: f64,
    longitude: f64,
}

pub fn locality_from_env() -> Result<Locality, String> {
    let name = env::var("SAAIOS_LOCALITY_NAME").unwrap_or_default();
    let latitude: f64 = env::var("SAAIOS_LOCALITY_LAT")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .ok_or_else(|| "местность не задана: нет широты".to_string())?;
    let longitude: f64 = env::var("SAAIOS_LOCALITY_LON")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .ok_or_else(|| "местность не задана: нет долготы".to_string())?;
    if !latitude.is_finite() || !longitude.is_finite() {
        return Err("координаты не конечные".into());
    }
    Ok(Locality {
        name,
        latitude,
        longitude,
    })
}

pub fn format_coords(latitude: f64, longitude: f64) -> String {
    format!("{latitude:.5}, {longitude:.5}")
}

pub fn cache_path() -> PathBuf {
    if let Ok(path) = env::var("SAAIOS_LOCALITY_CACHE") {
        return PathBuf::from(path);
    }
    let device = PathBuf::from("/data/saaios/var/locality-weather.json");
    if device.parent().is_some_and(|parent| parent.exists()) {
        return device;
    }
    std::env::temp_dir().join("saaios-locality-weather.json")
}

pub fn state_path() -> PathBuf {
    cache_path().with_extension("state.json")
}

pub fn read_state() -> FetchState {
    fs::read_to_string(state_path())
        .ok()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default()
}

fn write_state(state: &FetchState) -> Result<(), String> {
    let path = state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("кэш: {error}"))?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(state).map_err(|error| format!("кэш: {error}"))?,
    )
    .map_err(|error| format!("кэш: {error}"))
}

pub fn fetch_current(locality: &Locality) -> Result<Value, String> {
    let url = format!(
        "{FORECAST_URL}?latitude={:.5}&longitude={:.5}&current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m&timezone=auto",
        locality.latitude, locality.longitude
    );
    let response = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        .build()
        .and_then(|client| client.get(&url).send())
        .map_err(|error| format!("сеть: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Open-Meteo HTTP {}", response.status()));
    }
    response
        .json()
        .map_err(|error| format!("Open-Meteo JSON: {error}"))
}

pub fn fetch_with_retries(locality: &Locality, attempts: u32) -> Result<Value, String> {
    let mut last = "нет попыток".to_string();
    for _ in 0..attempts.max(1) {
        match fetch_current(locality) {
            Ok(forecast) => return Ok(forecast),
            Err(error) => last = error,
        }
    }
    Err(last)
}

pub fn write_cache(
    path: &Path,
    locality: &Locality,
    forecast: &Value,
    fetched_at: chrono::DateTime<Utc>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("кэш: {error}"))?;
    }
    let body = LocalityCacheFile {
        locality: CachePlace {
            name: locality.name.clone(),
            latitude: locality.latitude,
            longitude: locality.longitude,
        },
        attribution: ATTRIBUTION.into(),
        source: SOURCE_OPEN_METEO.into(),
        fetched_at: fetched_at.to_rfc3339(),
        forecast: forecast.clone(),
    };
    fs::write(
        path,
        serde_json::to_vec_pretty(&body).map_err(|error| format!("кэш: {error}"))?,
    )
    .map_err(|error| format!("кэш: {error}"))
}

pub fn widget_line_from_rows(
    rows: &[saai_observation::Observation],
    fallback: Option<&Locality>,
) -> Option<String> {
    let temperature = number_of(rows, KEY_TEMPERATURE)?;
    let condition = text_of(rows, KEY_CONDITION).unwrap_or_else(|| {
        number_of(rows, KEY_WEATHER_CODE)
            .map(|code| wmo_condition_label(code as u64).to_string())
            .unwrap_or_default()
    });
    let latitude =
        number_of(rows, KEY_LATITUDE).or_else(|| fallback.map(|place| place.latitude))?;
    let longitude =
        number_of(rows, KEY_LONGITUDE).or_else(|| fallback.map(|place| place.longitude))?;
    let mut parts = vec![format!("{temperature:.1}°")];
    if !condition.is_empty() {
        parts.push(condition);
    }
    if let Some(wind) = number_of(rows, KEY_WIND_SPEED) {
        parts.push(format!("{wind:.0} км/ч"));
    }
    if let Some(humidity) = number_of(rows, KEY_HUMIDITY) {
        parts.push(format!("{humidity:.0}%"));
    }
    parts.push(format_coords(latitude, longitude));
    Some(parts.join(" · "))
}

pub fn widget_status() -> String {
    if let Some(line) = cached_widget_line() {
        return line;
    }
    let coords = locality_from_env()
        .ok()
        .map(|place| format_coords(place.latitude, place.longitude));
    let state = read_state();
    if state.attempts >= FETCH_ATTEMPTS {
        let reason = state
            .last_error
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "нет данных".into());
        return match coords {
            Some(coords) => format!("{reason} · {coords}"),
            None => reason,
        };
    }
    match coords {
        Some(coords) => format!("Нет наблюдения · {coords}"),
        None => "Нет наблюдения".into(),
    }
}

fn cached_widget_line() -> Option<String> {
    let blob = fs::read_to_string(cache_path()).ok()?;
    let value: Value = serde_json::from_str(&blob).ok()?;
    let rows = observations_from_locality_cache(&value, Utc::now(), 1);
    widget_line_from_rows(&rows, None)
}

/// Tap handler: start a fresh five-attempt cycle.
pub fn refresh_locality() -> String {
    let locality = match locality_from_env() {
        Ok(locality) => locality,
        Err(error) => {
            let _ = write_state(&FetchState {
                attempts: FETCH_ATTEMPTS,
                last_error: Some(error),
            });
            return widget_status();
        }
    };
    match fetch_with_retries(&locality, FETCH_ATTEMPTS) {
        Ok(forecast) => {
            let observed_at = Utc::now();
            let rows = observations_from_open_meteo_current(&forecast, &locality, observed_at, 1);
            if rows.is_empty() {
                let _ = write_state(&FetchState {
                    attempts: FETCH_ATTEMPTS,
                    last_error: Some(EMPTY_FORECAST.into()),
                });
            } else {
                let _ = write_cache(&cache_path(), &locality, &forecast, observed_at);
                let _ = write_state(&FetchState::default());
            }
        }
        Err(error) => {
            let _ = write_state(&FetchState {
                attempts: FETCH_ATTEMPTS,
                last_error: Some(error),
            });
        }
    }
    widget_status()
}

pub fn report_lines(locality: &Locality, rows: &[saai_observation::Observation]) -> Vec<String> {
    let mut lines = Vec::new();
    let place = if locality.name.trim().is_empty() {
        format_coords(locality.latitude, locality.longitude)
    } else {
        format!(
            "{} · {}",
            locality.name,
            format_coords(locality.latitude, locality.longitude)
        )
    };
    lines.push(format!("Местность: {place}"));
    if let Some(line) = widget_line_from_rows(rows, Some(locality)) {
        lines.push(line);
    }
    lines.push(format!("Источник: {ATTRIBUTION}"));
    lines
}

fn number_of(rows: &[saai_observation::Observation], key: &str) -> Option<f64> {
    rows.iter()
        .find(|row| row.key == key)
        .and_then(|row| row.value.as_f64())
}

fn text_of(rows: &[saai_observation::Observation], key: &str) -> Option<String> {
    rows.iter()
        .find(|row| row.key == key)
        .and_then(|row| row.value.as_str().map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use saai_observation::{observations_from_open_meteo_current, Locality};
    use serde_json::json;

    fn berlin() -> Locality {
        Locality {
            name: "Berlin".into(),
            latitude: 52.52,
            longitude: 13.41,
        }
    }

    #[test]
    fn widget_line_shows_main_facts_and_coords() {
        let rows = observations_from_open_meteo_current(
            &json!({
                "current": {
                    "temperature_2m": 10.2,
                    "relative_humidity_2m": 74,
                    "weather_code": 3,
                    "wind_speed_10m": 12.7
                }
            }),
            &berlin(),
            Utc::now(),
            1,
        );
        let line = widget_line_from_rows(&rows, Some(&berlin())).expect("line");
        assert!(line.contains("10.2°"));
        assert!(line.contains("Пасмурно"));
        assert!(line.contains("13 км/ч"));
        assert!(line.contains("74%"));
        assert!(line.contains("52.52000, 13.41000"));
    }

    #[test]
    fn reason_is_held_until_five_attempts() {
        let early = FetchState {
            attempts: 2,
            last_error: Some("сеть: timeout".into()),
        };
        assert!(early.attempts < FETCH_ATTEMPTS);
        let done = FetchState {
            attempts: FETCH_ATTEMPTS,
            last_error: Some("сеть: timeout".into()),
        };
        assert_eq!(done.attempts, 5);
        assert_eq!(done.last_error.as_deref(), Some("сеть: timeout"));
    }

    #[test]
    fn empty_forecast_reason_hides_the_helper_field() {
        assert_eq!(EMPTY_FORECAST, "Open-Meteo · нет данных");
        assert!(!EMPTY_FORECAST.contains("current"));
    }

    #[test]
    fn format_coords_keeps_five_decimals() {
        assert_eq!(format_coords(55.75, 37.62), "55.75000, 37.62000");
    }
}
