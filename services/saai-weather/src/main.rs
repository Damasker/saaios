//! Locality weather from Open-Meteo. Place is configured, never guessed.

use chrono::Utc;
use saai_observation::observations_from_open_meteo_current;
use saai_weather::{
    cache_path, fetch_with_retries, locality_from_env, report_lines, write_cache, EMPTY_FORECAST,
    FETCH_ATTEMPTS,
};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("saai-weather: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut name = env::var("SAAIOS_LOCALITY_NAME").ok();
    let mut latitude: Option<f64> = env::var("SAAIOS_LOCALITY_LAT")
        .ok()
        .and_then(|raw| raw.parse().ok());
    let mut longitude: Option<f64> = env::var("SAAIOS_LOCALITY_LON")
        .ok()
        .and_then(|raw| raw.parse().ok());
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--name" => name = Some(required("--name", args.next())?),
            "--lat" => {
                latitude = Some(
                    required("--lat", args.next())?
                        .parse()
                        .map_err(|_| "широта не число".to_string())?,
                )
            }
            "--lon" => {
                longitude = Some(
                    required("--lon", args.next())?
                        .parse()
                        .map_err(|_| "долгота не число".to_string())?,
                )
            }
            "--help" | "-h" => {
                eprintln!(
                    "saai-weather --lat <широта> --lon <долгота> [--name <подпись>]\n\
                     пять попыток, затем причина; координаты места запроса печатаются"
                );
                return Ok(());
            }
            other => return Err(format!("неизвестный флаг {other}")),
        }
    }
    if let (Some(latitude), Some(longitude)) = (latitude, longitude) {
        env::set_var("SAAIOS_LOCALITY_LAT", latitude.to_string());
        env::set_var("SAAIOS_LOCALITY_LON", longitude.to_string());
    }
    if let Some(name) = name {
        env::set_var("SAAIOS_LOCALITY_NAME", name);
    }
    let locality = locality_from_env()?;
    let forecast = fetch_with_retries(&locality, FETCH_ATTEMPTS)?;
    let observed_at = Utc::now();
    let rows = observations_from_open_meteo_current(&forecast, &locality, observed_at, 1);
    if rows.is_empty() {
        return Err(EMPTY_FORECAST.into());
    }
    let path = cache_path();
    write_cache(&path, &locality, &forecast, observed_at)?;
    println!("кэш {}", path.display());
    for line in report_lines(&locality, &rows) {
        println!("{line}");
    }
    Ok(())
}

fn required(flag: &str, value: Option<String>) -> Result<String, String> {
    value.ok_or_else(|| format!("{flag} ждёт значение"))
}
