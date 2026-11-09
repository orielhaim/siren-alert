use crate::{Runtime, publish, settings_only};
use rodio::{Decoder, DeviceSinkBuilder, Player};
use serde::{Deserialize, Serialize};
use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::{Mutex, mpsc},
    time::Duration,
};
use tauri::{Emitter, Manager};
const MAX_BYTES: u64 = 20 * 1024 * 1024;
const EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "m4a", "aac", "webm", "flac"];
#[derive(Clone, Serialize, Deserialize)]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub custom: bool,
}
#[derive(Serialize, Deserialize)]
struct Record {
    id: String,
    name: String,
    extension: String,
}
pub struct Library(pub Mutex<()>);
enum Command {
    Play {
        bytes: Vec<u8>,
        priority: u8,
        preview: bool,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Stop,
}
pub struct Audio(mpsc::Sender<Command>);
impl Audio {
    pub fn start(app: tauri::AppHandle) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut output = None;
            let mut playing: Option<Player> = None;
            let mut priority = 0;
            loop {
                let command = match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(command) => command,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if playing.as_ref().is_some_and(|sink| sink.empty()) {
                            playing = None;
                            priority = 0;
                            let _ = app.emit("audio-playing", false);
                        }
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
                match command {
                    Command::Stop => {
                        playing = None;
                        priority = 0;
                        let _ = app.emit("audio-playing", false);
                    }
                    Command::Play {
                        bytes,
                        priority: next,
                        preview,
                        reply,
                    } => {
                        if !preview
                            && playing.as_ref().is_some_and(|s| !s.empty())
                            && priority >= next
                        {
                            let _ = reply.send(Ok(()));
                            continue;
                        }
                        let result = (|| {
                            let source = Decoder::try_from(Cursor::new(bytes))
                                .map_err(|_| "This audio file could not be decoded".to_owned())?;
                            if output.is_none() {
                                output =
                                    Some(DeviceSinkBuilder::open_default_sink().map_err(|_| {
                                        "No audio output is available. Check your sound device."
                                            .to_owned()
                                    })?);
                            }
                            let sink = Player::connect_new(output.as_ref().unwrap().mixer());
                            sink.set_volume(1.0);
                            sink.append(source);
                            playing = Some(sink);
                            priority = if preview { 0 } else { next };
                            let _ = app.emit("audio-playing", true);
                            Ok(())
                        })();
                        if let Err(ref e) = result {
                            let _ = app.emit("audio-error", e);
                        }
                        let _ = reply.send(result);
                    }
                }
            }
        });
        Self(tx)
    }
    fn play(&self, bytes: Vec<u8>, priority: u8, preview: bool) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.0
            .send(Command::Play {
                bytes,
                priority,
                preview,
                reply: tx,
            })
            .map_err(|_| "Audio engine stopped")?;
        rx.recv_timeout(Duration::from_secs(8))
            .map_err(|_| "Audio device did not respond")?
    }
    pub fn stop(&self) {
        let _ = self.0.send(Command::Stop);
    }
}
fn dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("sounds"))
        .map_err(|e| e.to_string())
}
fn valid_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|u| u.to_string() == id)
}
fn record(root: &Path, id: &str) -> Result<Record, String> {
    if !valid_id(id) {
        return Err("Unknown sound".into());
    }
    let r: Record = serde_json::from_slice(
        &std::fs::read(root.join(format!("{id}.json"))).map_err(|_| "Sound not found")?,
    )
    .map_err(|_| "Invalid sound record")?;
    if r.id != id || !EXTENSIONS.contains(&r.extension.as_str()) {
        return Err("Invalid sound record".into());
    }
    Ok(r)
}
fn builtin(id: &str) -> Option<&'static [u8]> {
    match id {
        "siren1" => Some(include_bytes!("../../public/sounds/siren1.mp3")),
        "siren2" => Some(include_bytes!("../../public/sounds/siren2.mp3")),
        "siren3" => Some(include_bytes!("../../public/sounds/siren3.mp3")),
        "siren4" => Some(include_bytes!("../../public/sounds/siren4.mp3")),
        _ => None,
    }
}
fn bytes(app: &tauri::AppHandle, id: &str) -> Result<Vec<u8>, String> {
    if let Some(b) = builtin(id) {
        return Ok(b.to_vec());
    }
    let root = dir(app)?;
    let r = record(&root, id)?;
    let file = std::fs::File::open(root.join(format!("{id}.{}", r.extension)))
        .map_err(|_| "Sound file is missing")?;
    let mut bytes = vec![];
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    validate(&bytes)?;
    Ok(bytes)
}
fn validate(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_BYTES {
        return Err("Choose an audio file up to 20 MB".into());
    }
    let mut decoder = Decoder::try_from(Cursor::new(bytes.to_vec()))
        .map_err(|_| "Unsupported or damaged audio file")?;
    if decoder.next().is_none() {
        return Err("This audio file is empty".into());
    }
    Ok(())
}
pub fn exists(app: &tauri::AppHandle, id: &str) -> bool {
    id == "disabled" || builtin(id).is_some() || dir(app).is_ok_and(|d| record(&d, id).is_ok())
}
#[tauri::command]
pub fn list_sounds(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Vec<Sound>, String> {
    settings_only(&window)?;
    let library = app.state::<Library>();
    let _guard = library.0.lock().unwrap();
    let mut sounds = (1..=4)
        .map(|i| Sound {
            id: format!("siren{i}"),
            name: format!("Siren {i}"),
            custom: false,
        })
        .collect::<Vec<_>>();
    let root = dir(&app)?;
    if root.exists() {
        for file in std::fs::read_dir(&root).map_err(|e| e.to_string())? {
            let file = file.map_err(|e| e.to_string())?;
            if file.path().extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if let Some(id) = file.path().file_stem().and_then(|s| s.to_str())
                && let Ok(r) = record(&root, id)
            {
                sounds.push(Sound {
                    id: r.id,
                    name: r.name,
                    custom: true,
                });
            }
        }
    }
    sounds[4..].sort_by(|a, b| a.name.cmp(&b.name));
    Ok(sounds)
}
#[tauri::command]
pub async fn import_sound(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Option<Sound>, String> {
    settings_only(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_parent(&window)
            .set_title("Add alert sound")
            .add_filter("Audio", EXTENSIONS)
            .pick_file()
        else {
            return Ok(None);
        };
        let library = app.state::<Library>();
        let _guard = library.0.lock().unwrap();
        import_file(&dir(&app)?, &path).map(Some)
    })
    .await
    .map_err(|e| e.to_string())?
}
fn import_file(root: &Path, path: &Path) -> Result<Sound, String> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !EXTENSIONS.contains(&extension.as_str()) {
        return Err("Unsupported audio format".into());
    }
    let file = std::fs::File::open(path).map_err(|_| "Cannot open audio file")?;
    let mut data = vec![];
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    validate(&data)?;
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().chars().take(100).collect())
        .unwrap_or_else(|| "Custom sound".into());
    let r = Record {
        id: id.clone(),
        name,
        extension,
    };
    let asset = root.join(format!("{id}.{}", r.extension));
    std::fs::write(&asset, data).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::write(
        root.join(format!("{id}.json")),
        serde_json::to_vec(&r).map_err(|e| e.to_string())?,
    ) {
        let _ = std::fs::remove_file(asset);
        return Err(e.to_string());
    }
    Ok(Sound {
        id,
        name: r.name,
        custom: true,
    })
}
#[tauri::command]
pub fn remove_sound(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: String,
) -> Result<(), String> {
    settings_only(&window)?;
    let library = app.state::<Library>();
    let _guard = library.0.lock().unwrap();
    let state = app.state::<Runtime>();
    let view = state.view.lock().unwrap();
    if view.settings.warning_sound == id || view.settings.alert_sound == id {
        return Err("Choose and save another sound before removing this one".into());
    }
    let root = dir(&app)?;
    let r = record(&root, &id)?;
    std::fs::remove_file(root.join(format!("{id}.{}", r.extension))).map_err(|e| e.to_string())?;
    std::fs::remove_file(root.join(format!("{id}.json"))).map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn rename_sound(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: String,
    name: String,
) -> Result<(), String> {
    settings_only(&window)?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err("Enter a sound name up to 100 characters".into());
    }
    let library = app.state::<Library>();
    let _guard = library.0.lock().unwrap();
    let root = dir(&app)?;
    let mut r = record(&root, &id)?;
    r.name = name.into();
    let temp = root.join(format!("{id}.tmp"));
    std::fs::write(&temp, serde_json::to_vec(&r).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    std::fs::rename(temp, root.join(format!("{id}.json"))).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn preview_sound(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: String,
) -> Result<(), String> {
    settings_only(&window)?;
    if app
        .state::<Runtime>()
        .view
        .lock()
        .unwrap()
        .alerts
        .iter()
        .any(|a| !a.is_test && a.kind != "endAlert")
    {
        return Err("Sound previews are unavailable during a live alert".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if id == "disabled" {
            app.state::<Audio>().stop();
            return Ok(());
        }
        app.state::<Audio>().play(bytes(&app, &id)?, 2, true)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn stop_sound(app: tauri::AppHandle) {
    app.state::<Audio>().stop();
}
pub fn alert(app: &tauri::AppHandle, warning: bool) {
    let (settings, live) = {
        let state = app.state::<Runtime>();
        let view = state.view.lock().unwrap();
        (
            view.settings.clone(),
            view.alerts
                .iter()
                .any(|a| !a.is_test && a.kind != "newsFlash" && a.kind != "endAlert"),
        )
    };
    if !settings.sound {
        return;
    }
    let id = if warning {
        settings.warning_sound
    } else {
        settings.alert_sound
    };
    if id == "disabled" || (warning && live) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = bytes(&app, &id).and_then(|b| {
            app.state::<Audio>()
                .play(b, if warning { 0 } else { 1 }, false)
        });
        if let Err(e) = result {
            let _ = app.emit("audio-error", e);
        }
        publish(&app);
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_copy_and_decode_audio() {
        let root = std::env::temp_dir().join(format!("siren-sound-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let source = root.join("source.mp3");
        std::fs::write(&source, builtin("siren2").unwrap()).unwrap();
        let target = root.join("library");
        let sound = import_file(&target, &source).unwrap();
        std::fs::remove_file(source).unwrap();
        let meta = record(&target, &sound.id).unwrap();
        assert_eq!(meta.name, "source");
        validate(&std::fs::read(target.join(format!("{}.mp3", sound.id))).unwrap()).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_invalid_files_and_ids() {
        assert!(validate(b"not audio").is_err());
        assert!(!valid_id("../../settings"));
        assert!(!valid_id("siren1"));
    }
    #[test]
    fn every_bundled_sound_decodes() {
        for id in ["siren1", "siren2", "siren3", "siren4"] {
            validate(builtin(id).unwrap()).unwrap();
        }
    }
    #[test]
    #[ignore = "requires a native audio output device"]
    fn native_audio_output() {
        use rodio::Source;
        let stream = DeviceSinkBuilder::open_default_sink().expect("Default output device");
        let sink = Player::connect_new(stream.mixer());
        sink.set_volume(0.1);
        sink.append(
            Decoder::try_from(Cursor::new(builtin("siren1").unwrap().to_vec()))
                .unwrap()
                .take_duration(Duration::from_millis(150)),
        );
        sink.sleep_until_end();
        assert!(sink.empty());
    }
}
