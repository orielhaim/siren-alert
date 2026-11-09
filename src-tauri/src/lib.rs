mod catalog;
mod sounds;
use serde::{Deserialize, Serialize};
use sioc::prelude::*;
use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
    time::Duration,
};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tokio::sync::watch;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Style {
    CornerLeft,
    CornerRight,
    Center,
    Fullscreen,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Language {
    En,
    He,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    style: Style,
    cities: Vec<String>,
    sound: bool,
    warning_sound: String,
    alert_sound: String,
    countdown: bool,
    language: Language,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            style: Style::CornerRight,
            cities: vec![],
            sound: true,
            warning_sound: "disabled".into(),
            alert_sound: "siren1".into(),
            countdown: true,
            language: Language::En,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Alert {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    cities: Vec<String>,
    title: Option<String>,
    instructions: Option<String>,
    #[serde(default)]
    is_test: bool,
    #[serde(default)]
    received_at: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    settings: Settings,
    status: String,
    alerts: Vec<Alert>,
    has_key: bool,
    autostart: bool,
}
struct Runtime {
    view: Mutex<View>,
    restart: watch::Sender<u64>,
    filters: watch::Sender<u64>,
    catalog: Mutex<Vec<catalog::City>>,
    catalog_load: tokio::sync::Mutex<()>,
}
fn show_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new("com.orielhaim.siren-alert", "api-key")
        .map_err(|_| "Cannot open the system credential store".into())
}
fn credential() -> Result<Option<String>, String> {
    match entry()?.get_password() {
        Ok(s) => Ok(Some(s)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("Cannot read the system credential store".into()),
    }
}
fn publish(app: &tauri::AppHandle) {
    if let Ok(v) = app.state::<Runtime>().view.lock() {
        let _ = app.emit("state", &*v);
    }
}
fn status(app: &tauri::AppHandle, s: &str) {
    app.state::<Runtime>().view.lock().unwrap().status = s.into();
    publish(app);
}
fn settings_only(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("Settings window required".into())
    }
}
#[tauri::command]
async fn check_update(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    install: bool,
) -> Result<String, String> {
    settings_only(&window)?;
    if !app.config().plugins.0.contains_key("updater") {
        return Ok("Updates will be available after the first signed release.".into());
    }
    use tauri_plugin_updater::UpdaterExt;
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    match update {
        None => Ok("You are using the latest version.".into()),
        Some(update) => {
            let version = update.version.clone();
            if install {
                update
                    .download_and_install(|_, _| {}, || {})
                    .await
                    .map_err(|e| e.to_string())?;
                app.restart();
            }
            Ok(format!("Update {version} is available."))
        }
    }
}
#[tauri::command]
fn open_help(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    portal: bool,
) -> Result<(), String> {
    settings_only(&window)?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(
            if portal {
                "https://siren.co.il"
            } else {
                "https://docs.siren.co.il/getting-started/access/"
            },
            None::<&str>,
        )
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn open_repository(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    settings_only(&window)?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url("https://github.com/orielhaim/siren-alert", None::<&str>)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn get_state(app: tauri::AppHandle) -> View {
    app.state::<Runtime>().view.lock().unwrap().clone()
}
#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    mut settings: Settings,
) -> Result<(), String> {
    settings_only(&window)?;
    settings.cities = settings
        .cities
        .iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    settings.cities.sort();
    settings.cities.dedup();
    if settings.cities.len() > 500 || settings.cities.iter().any(|s| s.len() > 300) {
        return Err("Invalid location selection".into());
    }
    let library = app.state::<sounds::Library>();
    let _library_guard = library.0.lock().unwrap();
    if !sounds::exists(&app, &settings.warning_sound)
        || !sounds::exists(&app, &settings.alert_sound)
    {
        return Err("Choose an available sound".into());
    }
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join("settings.json");
    let temp = dir.join("settings.tmp");
    std::fs::write(
        &temp,
        serde_json::to_vec(&settings).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(temp, file).map_err(|e| e.to_string())?;
    let locations_changed =
        app.state::<Runtime>().view.lock().unwrap().settings.cities != settings.cities;
    app.state::<Runtime>().view.lock().unwrap().settings = settings;
    if locations_changed {
        let state = app.state::<Runtime>();
        {
            let mut view = state.view.lock().unwrap();
            let selected = view.settings.cities.clone();
            if !selected.is_empty() {
                for alert in &mut view.alerts {
                    alert.cities.retain(|c| selected.contains(c));
                }
                view.alerts.retain(|a| !a.cities.is_empty());
            }
        }
        state.filters.send_modify(|v| *v += 1);
    }
    if !app.state::<Runtime>().view.lock().unwrap().settings.sound {
        app.state::<sounds::Audio>().stop();
    }
    publish(&app);
    Ok(())
}
#[tauri::command]
async fn set_key(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    key: String,
) -> Result<(), String> {
    settings_only(&window)?;
    let key = key.trim().to_owned();
    if key.is_empty() {
        return Err("Enter a private Siren API key".into());
    }
    http()
        .get("https://api.siren.co.il/active")
        .header("X-API-Key", &key)
        .send()
        .await
        .map_err(|_| "Cannot reach Siren. Check your connection.")?
        .error_for_status()
        .map_err(|_| "Siren rejected this key or is temporarily unavailable.")?;
    entry()?
        .set_password(&key)
        .map_err(|_| "Could not save the key in the system credential store")?;
    app.state::<Runtime>().view.lock().unwrap().has_key = true;
    app.state::<Runtime>().restart.send_modify(|v| *v += 1);
    Ok(())
}
#[tauri::command]
fn set_autostart(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    enabled: bool,
) -> Result<(), String> {
    settings_only(&window)?;
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|e| e.to_string())?;
    app.state::<Runtime>().view.lock().unwrap().autostart = enabled;
    publish(&app);
    Ok(())
}
#[tauri::command]
fn dismiss(app: tauri::AppHandle) -> Result<(), String> {
    app.state::<sounds::Audio>().stop();
    app.get_webview_window("alert")
        .ok_or("Alert window unavailable")?
        .hide()
        .map_err(|e| e.to_string())
}
fn present(app: &tauri::AppHandle) -> Result<(), String> {
    let style = app
        .state::<Runtime>()
        .view
        .lock()
        .unwrap()
        .settings
        .style
        .clone();
    let w = app
        .get_webview_window("alert")
        .ok_or("Alert window unavailable")?;
    let m = w
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No display available")?;
    let area = m.work_area();
    let scale = m.scale_factor();
    w.set_fullscreen(false).map_err(|e| e.to_string())?;
    let (width, height) = if style == Style::Fullscreen {
        (
            area.size.width as f64 / scale,
            area.size.height as f64 / scale,
        )
    } else if style == Style::Center {
        (560.0, 460.0)
    } else {
        (420.0, 340.0)
    };
    let width = width.min(area.size.width as f64 / scale);
    let height = height.min(area.size.height as f64 / scale);
    w.set_size(tauri::LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    let x = area.position.x as f64 / scale;
    let y = area.position.y as f64 / scale;
    let aw = area.size.width as f64 / scale;
    let ah = area.size.height as f64 / scale;
    let (px, py) = match style {
        Style::CornerLeft => (x + 16.0, y + 16.0),
        Style::CornerRight => (x + aw - width - 16.0, y + 16.0),
        _ => (x + (aw - width) / 2.0, y + (ah - height) / 2.0),
    };
    w.set_position(tauri::LogicalPosition::new(px, py))
        .map_err(|e| e.to_string())?;
    if style == Style::Fullscreen {
        w.set_fullscreen(true).map_err(|e| e.to_string())?;
    }
    w.show().map_err(|e| e.to_string())
}
#[tauri::command]
fn preview_alert(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    settings_only(&window)?;
    let mut v = app.state::<Runtime>().view.lock().unwrap().clone();
    if !v.alerts.is_empty() && v.alerts.iter().any(|a| !a.is_test) {
        return Err("A live alert is being displayed".into());
    }
    v.alerts = vec![Alert {
        id: format!(
            "preview-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ),
        kind: "missiles".into(),
        cities: vec!["התרעת ניסיון".into()],
        title: Some("בדיקת התרעה".into()),
        instructions: Some("זוהי בדיקה בלבד. אין צורך בפעולה.".into()),
        received_at: Some(chrono::Utc::now().to_rfc3339()),
        is_test: true,
    }];
    app.state::<Runtime>().view.lock().unwrap().alerts = v.alerts;
    publish(&app);
    sounds::alert(&app, false);
    present(&app)
}
fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .expect("HTTP client")
}
fn apply_alert(current: &mut Vec<Alert>, mut alert: Alert, selected: &[String]) {
    if !selected.is_empty() {
        alert.cities.retain(|c| selected.contains(c));
    }
    if alert.cities.is_empty() {
        return;
    }
    for previous in current.iter_mut() {
        if alert.kind == "endAlert"
            || (alert.kind != "newsFlash" && previous.kind == "newsFlash")
            || alert.kind == previous.kind
            || previous.kind == "endAlert"
        {
            previous.cities.retain(|c| !alert.cities.contains(c));
        }
    }
    current.retain(|a| !a.cities.is_empty() && !a.is_test);
    if alert.kind == "endAlert" {
        alert.received_at = Some(chrono::Utc::now().to_rfc3339());
    }
    current.push(alert);
}
async fn reconcile(app: &tauri::AppHandle, key: &str) -> Result<bool, String> {
    let snapshot = http()
        .get("https://api.siren.co.il/active")
        .header("X-API-Key", key)
        .send()
        .await
        .map_err(|_| "Snapshot unavailable")?
        .error_for_status()
        .map_err(|_| "Snapshot rejected")?
        .json::<HashMap<String, Vec<String>>>()
        .await
        .map_err(|_| "Invalid snapshot")?;
    let state = app.state::<Runtime>();
    let mut v = state.view.lock().unwrap();
    let selected = v.settings.cities.clone();
    let before = serde_json::to_string(&v.alerts).unwrap();
    let mut alerts = vec![];
    for (kind, cities) in snapshot {
        for city in cities {
            if !selected.is_empty() && !selected.contains(&city) {
                continue;
            }
            if let Some(existing) = v
                .alerts
                .iter()
                .find(|a| !a.is_test && a.kind == kind && a.cities.contains(&city))
            {
                let mut a = existing.clone();
                a.cities = vec![city];
                alerts.push(a);
            } else {
                alerts.push(Alert {
                    id: format!("snapshot-{kind}-{city}"),
                    kind: kind.clone(),
                    cities: vec![city],
                    title: Some(format!("התרעה פעילה · {kind}")),
                    instructions: None,
                    received_at: None,
                    is_test: false,
                });
            }
        }
    }
    let mut grouped: HashMap<String, Alert> = HashMap::new();
    for a in alerts {
        if let Some(previous) = grouped.get_mut(&a.id) {
            previous.cities.extend(a.cities);
        } else {
            grouped.insert(a.id.clone(), a);
        }
    }
    let mut alerts: Vec<Alert> = grouped.into_values().collect();
    for a in &mut alerts {
        a.cities.sort();
        a.cities.dedup();
    }
    alerts.sort_by(|a, b| a.id.cmp(&b.id).then(a.cities.cmp(&b.cities)));
    v.alerts = alerts;
    Ok(before != serde_json::to_string(&v.alerts).unwrap())
}
async fn refresh_active(app: &tauri::AppHandle, key: &str) -> Result<(), String> {
    if reconcile(app, key).await? {
        publish(app);
        if app
            .state::<Runtime>()
            .view
            .lock()
            .unwrap()
            .alerts
            .is_empty()
        {
            app.state::<sounds::Audio>().stop();
            if let Some(window) = app.get_webview_window("alert") {
                let _ = window.hide();
            }
        } else {
            present(app)?;
        }
    }
    Ok(())
}
async fn session(app: tauri::AppHandle, key: String) -> Result<(), String> {
    let client = ClientBuilder::new(url::Url::parse("https://api.siren.co.il").unwrap())
        .open()
        .map_err(|e| e.to_string())?;
    let (_tx, mut rx) = client
        .connect_with("/", serde_json::json!({"apiKey":key}).to_string())
        .await
        .map_err(|e| e.to_string())?;
    let mut seen = VecDeque::new();
    let mut filters = app.state::<Runtime>().filters.subscribe();
    let mut expiry = tokio::time::interval(Duration::from_secs(1));
    let mut connected = false;
    let mut refresh = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_secs(60),
        Duration::from_secs(60),
    );
    loop {
        let signal = if connected {
            tokio::select! {
                signal=rx.recv()=>signal,
                _=refresh.tick()=>{refresh_active(&app,&key).await?;continue;},
                _=filters.changed()=>{refresh_active(&app,&key).await?;continue;},
                _=expiry.tick()=>{let changed={let state=app.state::<Runtime>();let mut view=state.view.lock().unwrap();let before=view.alerts.len();view.alerts.retain(|a|a.kind!="endAlert" || a.received_at.as_deref().and_then(|s|chrono::DateTime::parse_from_rfc3339(s).ok()).is_none_or(|t|chrono::Utc::now().signed_duration_since(t).num_seconds()<30));view.alerts.len()!=before};if changed{publish(&app);if app.state::<Runtime>().view.lock().unwrap().alerts.is_empty(){let _=app.get_webview_window("alert").unwrap().hide();}}continue;}
            }
        } else {
            tokio::time::timeout(Duration::from_secs(25), rx.recv())
                .await
                .map_err(|_| "Connection timed out")?
        }
        .ok_or("Connection closed")?;
        match signal {
            Signal::Connect(_) => {
                connected = true;
                let catalog_app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = catalog::load(&catalog_app, false).await {
                        let _ = catalog_app.emit("catalog-error", e);
                    }
                });
                status(&app, "Syncing current alerts");
                reconcile(&app, &key).await?;
                status(&app, "Connected");
                if !app
                    .state::<Runtime>()
                    .view
                    .lock()
                    .unwrap()
                    .alerts
                    .is_empty()
                {
                    present(&app)?;
                }
            }
            Signal::Disconnect | Signal::ConnectError(_) => {
                return Err("Disconnected. Retrying automatically".into());
            }
            Signal::Event(event) => {
                let payload: serde_json::Value =
                    serde_json::from_str(&event.payload).map_err(|_| "Invalid event")?;
                if payload.get(0).and_then(|v| v.as_str()) == Some("error") {
                    return Err("Siren reported a subscription error".into());
                }
                if payload.get(0).and_then(|v| v.as_str()) != Some("alert") {
                    continue;
                }
                let alerts: Vec<Alert> =
                    serde_json::from_value(payload.get(1).cloned().unwrap_or_default())
                        .map_err(|_| "Invalid alert payload")?;
                let mut changed = false;
                let mut audio_kind = None;
                {
                    let state = app.state::<Runtime>();
                    let mut v = state.view.lock().unwrap();
                    let selected = v.settings.cities.clone();
                    for a in alerts {
                        if a.is_test || seen.contains(&a.id) {
                            continue;
                        }
                        seen.push_back(a.id.clone());
                        if seen.len() > 2048 {
                            seen.pop_front();
                        }
                        if selected.is_empty() || a.cities.iter().any(|c| selected.contains(c)) {
                            if a.kind != "endAlert" {
                                let warning = a.kind == "newsFlash";
                                if audio_kind != Some(false) {
                                    audio_kind = Some(warning);
                                }
                            }
                            apply_alert(&mut v.alerts, a, &selected);
                            changed = true;
                        }
                    }
                }
                if changed {
                    if let Some(warning) = audio_kind {
                        sounds::alert(&app, warning);
                    }
                    if app
                        .state::<Runtime>()
                        .view
                        .lock()
                        .unwrap()
                        .alerts
                        .iter()
                        .all(|a| a.kind == "endAlert")
                    {
                        app.state::<sounds::Audio>().stop();
                    }
                    publish(&app);
                    if !app
                        .state::<Runtime>()
                        .view
                        .lock()
                        .unwrap()
                        .alerts
                        .is_empty()
                    {
                        present(&app)?;
                    }
                }
            }
        }
    }
}
async fn monitor(app: tauri::AppHandle, mut changes: watch::Receiver<u64>) {
    loop {
        match credential() {
            Ok(Some(key)) => {
                status(&app, "Connecting");
                tokio::select! {r=session(app.clone(),key)=>{if let Err(e)=r {status(&app,&e)}},_=changes.changed()=>{continue}}
            }
            Ok(None) => status(&app, "Add an API key to connect"),
            Err(e) => status(&app, &e),
        }
        tokio::select! {_=tokio::time::sleep(Duration::from_secs(15))=>{},_=changes.changed()=>{}}
    }
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_settings(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .setup(|app| {
            if app.config().plugins.0.contains_key("updater") {
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;
            }
            let settings = app
                .path()
                .app_config_dir()
                .ok()
                .and_then(|d| std::fs::read(d.join("settings.json")).ok())
                .map(|b| serde_json::from_slice::<Settings>(&b))
                .transpose()?
                .unwrap_or_default();
            let (restart, rx) = watch::channel(0);
            let (filters, _) = watch::channel(0);
            app.manage(Runtime {
                view: Mutex::new(View {
                    settings,
                    status: "Starting".into(),
                    alerts: vec![],
                    has_key: credential().ok().flatten().is_some(),
                    autostart: app.autolaunch().is_enabled().unwrap_or(false),
                }),
                restart,
                filters,
                catalog: Mutex::new(catalog::read_cache(app.handle())),
                catalog_load: tokio::sync::Mutex::new(()),
            });
            app.manage(sounds::Library(Mutex::new(())));
            app.manage(sounds::Audio::start(app.handle().clone()));
            for config in &app.config().app.windows {
                tauri::WebviewWindowBuilder::from_config(app, config)?.build()?;
            }
            use tauri::{
                menu::{Menu, MenuItem},
                tray::TrayIconBuilder,
            };
            let open = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Siren Alert", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .tooltip("Siren Alert")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_settings(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => {
                        show_settings(app);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            if std::env::args().any(|a| a == "--background")
                && let Some(w) = app.get_webview_window("main")
            {
                w.hide()?;
            }
            tauri::async_runtime::spawn(monitor(app.handle().clone(), rx));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "alert"
                    && let Some(audio) = window.try_state::<sounds::Audio>()
                {
                    audio.stop();
                }
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            open_help,
            open_repository,
            check_update,
            catalog::list_cities,
            catalog::refresh_cities,
            catalog::get_countdowns,
            sounds::list_sounds,
            sounds::import_sound,
            sounds::remove_sound,
            sounds::rename_sound,
            sounds::preview_sound,
            sounds::stop_sound,
            save_settings,
            set_key,
            set_autostart,
            dismiss,
            preview_alert
        ])
        .run(tauri::generate_context!())
        .expect("Siren could not start");
}
#[cfg(test)]
mod tests {
    use super::*;
    fn a(kind: &str, cities: &[&str]) -> Alert {
        Alert {
            id: kind.into(),
            kind: kind.into(),
            cities: cities.iter().map(|s| s.to_string()).collect(),
            title: None,
            instructions: None,
            received_at: None,
            is_test: false,
        }
    }
    #[test]
    fn replacement_and_ending_are_city_scoped() {
        let mut v = vec![];
        apply_alert(&mut v, a("newsFlash", &["a", "b"]), &[]);
        apply_alert(&mut v, a("missiles", &["a"]), &[]);
        assert_eq!(v[0].cities, vec!["b"]);
        apply_alert(&mut v, a("endAlert", &["a"]), &[]);
        assert_eq!(
            v.iter().map(|a| a.kind.as_str()).collect::<Vec<_>>(),
            vec!["newsFlash", "endAlert"]
        );
    }
    #[test]
    fn existing_settings_gain_new_preferences_without_losing_city_filters() {
        let settings: Settings =
            serde_json::from_str(r#"{"style":"center","cities":["חיפה"],"sound":false}"#).unwrap();
        assert_eq!(settings.cities, vec!["חיפה"]);
        assert_eq!(settings.alert_sound, "siren1");
        assert_eq!(settings.warning_sound, "disabled");
        assert!(settings.countdown);
        assert!(!settings.sound);
    }
    #[test]
    fn filters_do_not_accept_other_cities() {
        let mut v = vec![];
        apply_alert(&mut v, a("missiles", &["b"]), &["a".into()]);
        assert!(v.is_empty());
    }
}

#[cfg(test)]
mod network_tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires access to Siren test service"]
    async fn siren_test_namespace_connects() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = ClientBuilder::new(url::Url::parse("https://api.siren.co.il").unwrap())
            .open()
            .unwrap();
        let (tx, mut rx) = client.connect("/test").await.unwrap();
        let connected = tokio::time::timeout(Duration::from_secs(25), async {
            while let Some(signal) = rx.recv().await {
                match signal {
                    Signal::Connect(_) => return true,
                    Signal::ConnectError(_) => return false,
                    _ => {}
                }
            }
            false
        })
        .await
        .unwrap();
        tx.disconnect().await;
        assert!(connected, "Siren test namespace rejected connection");
    }
}

#[cfg(all(test, windows))]
mod credential_upgrade_tests {
    #[test]
    #[ignore = "uses an isolated temporary Windows credential"]
    fn windows_credential_storage_preserves_v3_format() {
        let service = "com.orielhaim.siren-alert.upgrade-test";
        let user = uuid::Uuid::new_v4().to_string();
        let entry = keyring::Entry::new(service, &user).unwrap();
        let password = "siren-test-בדיקה";
        let legacy_blob: Vec<u8> = password.encode_utf16().flat_map(u16::to_le_bytes).collect();
        entry.set_secret(&legacy_blob).unwrap();
        let read = entry.get_password();
        let attributes = entry.inner.get_attributes();
        let write = entry.set_password(password);
        let bytes = entry.get_secret();
        entry.delete_credential().unwrap();
        assert_eq!(read.unwrap(), password);
        assert_eq!(
            attributes.unwrap()["target_name"],
            format!("{user}.{service}")
        );
        write.unwrap();
        assert_eq!(bytes.unwrap(), legacy_blob);
    }
}
