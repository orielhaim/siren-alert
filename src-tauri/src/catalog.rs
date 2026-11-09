use crate::{Runtime, settings_only};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Manager;
#[derive(Clone, Serialize, Deserialize)]
pub struct City {
    pub name: String,
    #[serde(default)]
    pub translations: HashMap<String, String>,
    pub countdown: Option<u32>,
}
#[derive(Deserialize)]
struct CatalogRow {
    name: String,
    translations: Option<HashMap<String, String>>,
    countdown: Option<i16>,
}
fn parse_rows(value: serde_json::Value) -> Result<Vec<City>, String> {
    let rows: Vec<CatalogRow> = serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| City {
            name: row.name,
            translations: row.translations.unwrap_or_default(),
            countdown: row.countdown.and_then(|n| u32::try_from(n).ok()),
        })
        .collect())
}
fn cache(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("cities.json"))
        .map_err(|e| e.to_string())
}
pub fn read_cache(app: &tauri::AppHandle) -> Vec<City> {
    cache(app)
        .ok()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
pub async fn load(app: &tauri::AppHandle, refresh: bool) -> Result<Vec<City>, String> {
    let state = app.state::<Runtime>();
    let _load = state.catalog_load.lock().await;
    let current = app.state::<Runtime>().catalog.lock().unwrap().clone();
    if !refresh && !current.is_empty() {
        return Ok(current);
    }
    let key = crate::credential()?.ok_or("Connect your API key first")?;
    let mut cities = vec![];
    let mut offset = 0;
    loop {
        let data: serde_json::Value = crate::http()
            .get("https://api.siren.co.il/data/cities")
            .header("X-API-Key", &key)
            .query(&[
                ("limit", "500".to_owned()),
                ("offset", offset.to_string()),
                ("include", "translations,countdown".to_owned()),
            ])
            .send()
            .await
            .map_err(|_| "Cannot load locations")?
            .error_for_status()
            .map_err(|_| "Siren rejected the locations request")?
            .json()
            .await
            .map_err(|_| "Invalid city catalog")?;
        let rows = parse_rows(data["data"].clone())
            .map_err(|e| format!("Cannot read city catalog: {e}"))?;
        if data["pagination"]["hasMore"].as_bool() != Some(true) {
            cities.extend(rows);
            break;
        }
        if rows.is_empty() || offset > 10000 {
            return Err("Invalid catalog pagination".into());
        }
        cities.extend(rows);
        offset += 500;
    }
    let path = cache(app)?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let temp = path.with_extension("tmp");
    std::fs::write(
        &temp,
        serde_json::to_vec(&cities).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(temp, path).map_err(|e| e.to_string())?;
    *app.state::<Runtime>().catalog.lock().unwrap() = cities.clone();
    let _ = tauri::Emitter::emit(app, "catalog-updated", ());
    Ok(cities)
}
#[tauri::command]
pub async fn list_cities(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Vec<City>, String> {
    settings_only(&window)?;
    load(&app, false).await
}
#[tauri::command]
pub async fn refresh_cities(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Vec<City>, String> {
    settings_only(&window)?;
    load(&app, true).await
}
#[tauri::command]
pub fn get_countdowns(app: tauri::AppHandle) -> HashMap<String, u32> {
    let mut values: HashMap<String, u32> = app
        .state::<Runtime>()
        .catalog
        .lock()
        .unwrap()
        .iter()
        .filter_map(|c| c.countdown.map(|d| (c.name.clone(), d)))
        .collect();
    values.insert("התרעת ניסיון".into(), 90);
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nullable_city_metadata_is_accepted() {
        let cities=parse_rows(serde_json::json!([{"name":"חיפה","translations":null,"countdown":null},{"name":"ירושלים","translations":{"en":"Jerusalem"},"countdown":90}])).unwrap();
        assert_eq!(cities[0].name, "חיפה");
        assert!(cities[0].translations.is_empty());
        assert_eq!(cities[0].countdown, None);
        assert_eq!(cities[1].translations["en"], "Jerusalem");
        assert_eq!(cities[1].countdown, Some(90));
    }
}
