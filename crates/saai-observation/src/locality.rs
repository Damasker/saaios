//! Locality weather observations from an Open-Meteo forecast document.
//! This module does not open the network and does not invent a place.

use crate::model::{
    Observation, ObservationQuality, ObservationSource, ObservationSourceKind, ObservationSubject,
    SCHEMA_VERSION,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

pub const SOURCE_OPEN_METEO: &str = "open-meteo.forecast";
pub const KEY_LOCALITY_NAME: &str = "locality.name";
pub const KEY_LATITUDE: &str = "locality.latitude";
pub const KEY_LONGITUDE: &str = "locality.longitude";
pub const KEY_TEMPERATURE: &str = "locality.weather.temperature";
pub const KEY_HUMIDITY: &str = "locality.weather.humidity";
pub const KEY_WIND_SPEED: &str = "locality.weather.wind_speed";
pub const KEY_WEATHER_CODE: &str = "locality.weather.code";
pub const KEY_CONDITION: &str = "locality.weather.condition";
pub const KEY_ATTRIBUTION: &str = "locality.attribution";
pub const ATTRIBUTION: &str = "Open-Meteo (CC BY 4.0)";

/// Forecast models refresh on a longer cadence than CPU samples.
pub const LOCALITY_WEATHER_TTL_MS: u64 = 45 * 60 * 1000;

#[derive(Clone, Debug, PartialEq)]
pub struct Locality {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrentFile {
    current: Option<OpenMeteoCurrent>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: Option<f64>,
    relative_humidity_2m: Option<f64>,
    weather_code: Option<u64>,
    wind_speed_10m: Option<f64>,
}

pub fn wmo_condition_label(code: u64) -> &'static str {
    match code {
        0 => "Ясно",
        1 => "Преимущественно ясно",
        2 => "Переменная облачность",
        3 => "Пасмурно",
        45 | 48 => "Туман",
        51 | 53 | 55 | 56 | 57 => "Морось",
        61 | 63 | 65 | 66 | 67 => "Дождь",
        71 | 73 | 75 | 77 => "Снег",
        80 | 81 | 82 => "Ливень",
        85 | 86 => "Снегопад",
        95 | 96 | 99 => "Гроза",
        _ => "Неизвестно",
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct LocalityCache {
    pub locality: LocalityCachePlace,
    pub attribution: String,
    pub source: String,
    pub fetched_at: String,
    pub forecast: Value,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct LocalityCachePlace {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
}

pub fn observations_from_locality_cache(
    value: &Value,
    observed_at: DateTime<Utc>,
    sequence: u64,
) -> Vec<Observation> {
    let parsed: LocalityCache = match serde_json::from_value(value.clone()) {
        Ok(parsed) => parsed,
        Err(_) => return Vec::new(),
    };
    if parsed.source != SOURCE_OPEN_METEO {
        return Vec::new();
    }
    observations_from_open_meteo_current(
        &parsed.forecast,
        &Locality {
            name: parsed.locality.name,
            latitude: parsed.locality.latitude,
            longitude: parsed.locality.longitude,
        },
        observed_at,
        sequence,
    )
}

pub fn observations_from_open_meteo_current(
    value: &Value,
    locality: &Locality,
    observed_at: DateTime<Utc>,
    sequence: u64,
) -> Vec<Observation> {
    if !locality.latitude.is_finite() || !locality.longitude.is_finite() {
        return Vec::new();
    }
    let parsed: OpenMeteoCurrentFile = match serde_json::from_value(value.clone()) {
        Ok(parsed) => parsed,
        Err(_) => return Vec::new(),
    };
    let Some(current) = parsed.current else {
        return Vec::new();
    };

    let mut out = Vec::new();
    if !locality.name.trim().is_empty() {
        out.push(text_observation(
            KEY_LOCALITY_NAME,
            locality.name.trim(),
            None,
            observed_at,
            sequence,
        ));
    }
    out.push(number_observation(
        KEY_LATITUDE,
        locality.latitude,
        Some("degree"),
        observed_at,
        sequence,
    ));
    out.push(number_observation(
        KEY_LONGITUDE,
        locality.longitude,
        Some("degree"),
        observed_at,
        sequence,
    ));
    if let Some(temperature) = finite(current.temperature_2m) {
        out.push(number_observation(
            KEY_TEMPERATURE,
            temperature,
            Some("celsius"),
            observed_at,
            sequence,
        ));
    }
    if let Some(humidity) = finite(current.relative_humidity_2m) {
        out.push(number_observation(
            KEY_HUMIDITY,
            humidity,
            Some("percent"),
            observed_at,
            sequence,
        ));
    }
    if let Some(wind) = finite(current.wind_speed_10m) {
        out.push(number_observation(
            KEY_WIND_SPEED,
            wind,
            Some("km/h"),
            observed_at,
            sequence,
        ));
    }
    if let Some(code) = current.weather_code {
        out.push(number_observation(
            KEY_WEATHER_CODE,
            code as f64,
            Some("wmo"),
            observed_at,
            sequence,
        ));
        out.push(text_observation(
            KEY_CONDITION,
            wmo_condition_label(code),
            None,
            observed_at,
            sequence,
        ));
    }
    if !out.is_empty() {
        out.push(text_observation(
            KEY_ATTRIBUTION,
            ATTRIBUTION,
            None,
            observed_at,
            sequence,
        ));
    }
    out
}

fn finite(value: Option<f64>) -> Option<f64> {
    value.filter(|n| n.is_finite())
}

fn number_observation(
    key: &'static str,
    number: f64,
    unit: Option<&'static str>,
    observed_at: DateTime<Utc>,
    sequence: u64,
) -> Observation {
    Observation {
        schema: SCHEMA_VERSION,
        id: Uuid::new_v4(),
        subject: ObservationSubject::LocalDevice,
        key: key.into(),
        value: Value::from(number),
        unit: unit.map(str::to_string),
        source: ObservationSource {
            source_id: SOURCE_OPEN_METEO.into(),
            kind: ObservationSourceKind::Imported,
        },
        quality: ObservationQuality::Approximate,
        observed_at,
        valid_for_ms: LOCALITY_WEATHER_TTL_MS,
        sequence,
    }
}

fn text_observation(
    key: &'static str,
    text: &str,
    unit: Option<&'static str>,
    observed_at: DateTime<Utc>,
    sequence: u64,
) -> Observation {
    Observation {
        schema: SCHEMA_VERSION,
        id: Uuid::new_v4(),
        subject: ObservationSubject::LocalDevice,
        key: key.into(),
        value: Value::from(text),
        unit: unit.map(str::to_string),
        source: ObservationSource {
            source_id: SOURCE_OPEN_METEO.into(),
            kind: ObservationSourceKind::Imported,
        },
        quality: ObservationQuality::Approximate,
        observed_at,
        valid_for_ms: LOCALITY_WEATHER_TTL_MS,
        sequence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn berlin() -> Locality {
        Locality {
            name: "Berlin".into(),
            latitude: 52.52,
            longitude: 13.41,
        }
    }

    fn sample_current() -> Value {
        json!({
            "latitude": 52.52,
            "longitude": 13.419998,
            "current_units": {
                "temperature_2m": "°C",
                "relative_humidity_2m": "%",
                "weather_code": "wmo code",
                "wind_speed_10m": "km/h"
            },
            "current": {
                "time": "2026-10-11T09:45",
                "temperature_2m": 10.2,
                "relative_humidity_2m": 74,
                "weather_code": 3,
                "wind_speed_10m": 12.7
            }
        })
    }

    fn by_key<'a>(rows: &'a [Observation], key: &str) -> &'a Observation {
        rows.iter().find(|row| row.key == key).expect("key")
    }

    #[test]
    fn open_meteo_current_becomes_locality_observations() {
        let now = Utc::now();
        let rows = observations_from_open_meteo_current(&sample_current(), &berlin(), now, 4);
        assert_eq!(by_key(&rows, KEY_LOCALITY_NAME).value, json!("Berlin"));
        assert_eq!(by_key(&rows, KEY_LATITUDE).value, json!(52.52));
        assert_eq!(by_key(&rows, KEY_LONGITUDE).value, json!(13.41));
        let temperature = by_key(&rows, KEY_TEMPERATURE);
        assert_eq!(temperature.value, json!(10.2));
        assert_eq!(temperature.unit.as_deref(), Some("celsius"));
        assert_eq!(temperature.source.source_id, SOURCE_OPEN_METEO);
        assert_eq!(temperature.source.kind, ObservationSourceKind::Imported);
        assert_eq!(temperature.quality, ObservationQuality::Approximate);
        assert_eq!(by_key(&rows, KEY_HUMIDITY).value, json!(74.0));
        assert_eq!(by_key(&rows, KEY_WIND_SPEED).value, json!(12.7));
        assert_eq!(by_key(&rows, KEY_WEATHER_CODE).value, json!(3.0));
        assert_eq!(by_key(&rows, KEY_CONDITION).value, json!("Пасмурно"));
        assert_eq!(by_key(&rows, KEY_ATTRIBUTION).value, json!(ATTRIBUTION));
        assert!(rows.iter().all(|row| row.validate().is_ok()));
    }

    #[test]
    fn missing_current_or_invalid_place_invents_nothing() {
        let now = Utc::now();
        assert!(observations_from_open_meteo_current(&json!({}), &berlin(), now, 1).is_empty());
        assert!(observations_from_open_meteo_current(
            &sample_current(),
            &Locality {
                name: "Nowhere".into(),
                latitude: f64::NAN,
                longitude: 13.41,
            },
            now,
            1
        )
        .is_empty());
    }

    #[test]
    fn cache_without_open_meteo_source_is_dropped() {
        let now = Utc::now();
        let rows = observations_from_locality_cache(
            &json!({
                "locality": { "name": "Berlin", "latitude": 52.52, "longitude": 13.41 },
                "attribution": ATTRIBUTION,
                "source": "",
                "fetched_at": "2026-10-11T09:45:00Z",
                "forecast": sample_current()
            }),
            now,
            1,
        );
        assert!(rows.is_empty());
    }

    #[test]
    fn wmo_codes_stay_named_or_unknown() {
        assert_eq!(wmo_condition_label(0), "Ясно");
        assert_eq!(wmo_condition_label(61), "Дождь");
        assert_eq!(wmo_condition_label(1234), "Неизвестно");
    }
}
