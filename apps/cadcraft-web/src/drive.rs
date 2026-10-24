//! Saving to the student's own Google Drive. Every file CADCraft produces on the web (SAVE,
//! SAVEAS, exports) goes through [`deliver`]: it signs the user in with Google (the school
//! account, through Google Identity Services), puts the file in a "Disegni" folder in their Drive
//! with the `drive.file` scope (the app sees only files it created) and says where it went.
//! Whenever that fails (sign-in cancelled, no network, file too big) the file is downloaded
//! through the browser instead, so a drawing is never lost. `?nodrive` skips Drive altogether.
//!
//! `deliver` returns at once, so the engine counts the drawing as saved while the upload runs in
//! the background; the fallback download and the messages keep that honest.

use std::cell::RefCell;

use serde_json::{Value, json};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast as _, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys;

use crate::drivefmt;

/// OAuth client of the school's Google Cloud project. It is public by design: a browser app has
/// no secret, and Google only accepts sign-ins from the origins registered for it.
const CLIENT_ID: &str = "941646642229-v1amlshet3qalpu0qunksu7bee83dqc9.apps.googleusercontent.com";
/// Only the files this app creates; never the rest of the student's Drive.
const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
/// Pre-selects the school's accounts in Google's chooser.
const DOMAIN_HINT: &str = "smpavullo.com";
const GIS_URL: &str = "https://accounts.google.com/gsi/client";
const FOLDER_NAME: &str = "Disegni";
const DRIVE_URL: &str = "https://www.googleapis.com/drive/v3";
const FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
/// The only email domain a drawing may be sent to: the school's.
pub const SCHOOL_DOMAIN: &str = DOMAIN_HINT;
const UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
/// Drive's simple multipart upload is meant for files up to 5 MB; bigger ones are downloaded.
const MAX_UPLOAD_BYTES: usize = 5 * 1024 * 1024;
/// A token this close to its end is replaced before use.
const TOKEN_MARGIN_MS: f64 = 60_000.0;
const ERROR_TEXT_CHARS: usize = 200;

struct Token {
    value: String,
    expires_at_ms: f64,
}

/// How the file being saved is handed in, when the Hand in button started the save.
#[derive(Clone)]
enum Share {
    /// Share it with this teacher and let Drive email them.
    Teacher(String),
    /// Make a link anyone at the school can open, and show it to the student.
    Link,
}

/// A saved drawing: the token used and what Drive says about the file.
struct Saved {
    token: String,
    id: String,
    link: Option<String>,
}

thread_local! {
    static TOKEN: RefCell<Option<Token>> = const { RefCell::new(None) };
    /// Set while the Hand in button saves the drawing: what [`deliver`] does with the file it
    /// gets in that time besides keeping it in Drive.
    static SHARE: RefCell<Option<Share>> = const { RefCell::new(None) };
    /// A link ready to show: the file name and its link, taken by the app's next frame.
    static LINK: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
    /// Wakes the app when the background work has something to show.
    static CONTEXT: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
}

/// The "send to a teacher" hook ([`cadcraft_ui_egui::Services::send_to_teacher`]): the teacher's
/// address just before the drawing is saved, `None` right after.
pub fn request_share(teacher: Option<&str>) {
    SHARE.with(|s| *s.borrow_mut() = teacher.map(|t| Share::Teacher(t.to_string())));
}

/// The "save and copy link" hook ([`cadcraft_ui_egui::Services::share_link`]): `true` just before
/// the drawing is saved, `false` right after.
pub fn request_link(on: bool) {
    SHARE.with(|s| *s.borrow_mut() = on.then_some(Share::Link));
}

/// Called every frame with the app's egui context, so finished background work can wake it.
pub fn remember_context(ctx: &egui::Context) {
    CONTEXT.with(|c| {
        let mut c = c.borrow_mut();
        if c.is_none() {
            *c = Some(ctx.clone());
        }
    });
}

/// A link to a drawing that was just saved, if there is one: take it and show it.
pub fn take_link() -> Option<(String, String)> {
    LINK.with(|l| l.borrow_mut().take())
}

fn publish_link(name: &str, link: &str) {
    LINK.with(|l| *l.borrow_mut() = Some((name.to_string(), link.to_string())));
    CONTEXT.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.request_repaint();
        }
    });
}

/// The engine's delivery hook on the web ([`cadcraft_engine::cmd::file::set_deliver`]).
/// Drawings (DXF, DWG) go to Drive; images and PDFs are downloaded so they can be dragged into
/// the assignment.
pub fn deliver(name: &str, mime: &str, bytes: &[u8]) -> Result<(), String> {
    if drive_disabled() || bytes.len() > MAX_UPLOAD_BYTES || !drivefmt::is_drawing(name) {
        return crate::download::download(name, mime, bytes);
    }
    let share = SHARE.with(|s| s.borrow().clone());
    let (name, mime, bytes) = (name.to_string(), mime.to_string(), bytes.to_vec());
    wasm_bindgen_futures::spawn_local(async move {
        match save(&name, &mime, &bytes).await {
            Ok(saved) => match share {
                Some(Share::Teacher(teacher)) => match share_with_teacher(&saved.token, &saved.id, &name, &teacher).await {
                    Ok(()) => notify(&format!("Disegno inviato a {teacher}.\nÈ anche salvato nel tuo Google Drive, cartella «{FOLDER_NAME}»: {name}")),
                    Err(e) => notify(&format!("Il disegno è salvato nel tuo Drive ma NON è stato inviato a {teacher}: {e}")),
                },
                Some(Share::Link) => match link_for_school(&saved).await {
                    Ok(link) => {
                        copy_to_clipboard(&link);
                        publish_link(&name, &link);
                    }
                    Err(e) => notify(&format!("Il disegno è salvato nel tuo Drive ma non sono riuscito a fare il link: {e}")),
                },
                None => notify(&format!("Salvato nel tuo Google Drive, cartella «{FOLDER_NAME}»: {name}")),
            },
            Err(e) => {
                log::warn!("save to Drive failed: {e}");
                match crate::download::download(&name, &mime, &bytes) {
                    Ok(()) => notify(&format!("Non sono riuscito a salvare sul Drive ({e}). Il file è stato scaricato sul computer: {name}")),
                    Err(d) => notify(&format!("Salvataggio non riuscito. Drive: {e}. Download: {d}")),
                }
            }
        }
    });
    Ok(())
}

fn drive_disabled() -> bool {
    let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
    search.trim_start_matches('?').split('&').any(|p| p == "nodrive")
}

fn notify(message: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.alert_with_message(message);
    }
}

/// Try to put `text` on the clipboard. Browsers allow it only shortly after a click, so this can
/// quietly fail; the link window has a Copy button for that.
fn copy_to_clipboard(text: &str) {
    let Some(window) = web_sys::window() else { return };
    let navigator = window.navigator();
    let navigator: &JsValue = navigator.as_ref();
    let Some(clipboard) = field(navigator, "clipboard") else { return };
    let Some(write) = field(&clipboard, "writeText").and_then(|f| f.dyn_into::<js_sys::Function>().ok()) else { return };
    if let Ok(promise) = write.call1(&clipboard, &JsValue::from_str(text)) {
        // A refusal is a rejected promise: swallow it so it doesn't surface as an error.
        if let Ok(promise) = promise.dyn_into::<js_sys::Promise>() {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = JsFuture::from(promise).await;
            });
        }
    }
}

/// Put `bytes` in the "Disegni" folder as `name`, replacing the file of that name if it is there.
async fn save(name: &str, mime: &str, bytes: &[u8]) -> Result<Saved, String> {
    let token = access_token().await?;
    let folder = find_or_create_folder(&token).await?;
    let existing = find_file(&token, &folder, name).await?;
    let (url, method, meta) = match existing {
        Some(id) => (format!("{UPLOAD_URL}/{id}?uploadType=multipart&fields=id,webViewLink"), "PATCH", json!({ "name": name })),
        None => (format!("{UPLOAD_URL}?uploadType=multipart&fields=id,webViewLink"), "POST", json!({ "name": name, "parents": [folder] })),
    };
    let (content_type, body) = drivefmt::multipart(&meta, mime, bytes);
    let answer = request(&token, method, &url, Some(&content_type), Some(body)).await?;
    let id = drivefmt::created_id(&answer).ok_or_else(|| "Drive non ha restituito il file salvato".to_string())?;
    Ok(Saved { token, id, link: drivefmt::web_link(&answer) })
}

/// Let everyone at the school open the file through its link (not search for it, not outside the
/// school), and return that link. A pasted link to a private file would not open for the teacher.
async fn link_for_school(saved: &Saved) -> Result<String, String> {
    let url = format!("{FILES_URL}/{}/permissions?fields=id", saved.id);
    let body = json!({ "type": "domain", "role": "reader", "domain": SCHOOL_DOMAIN, "allowFileDiscovery": false });
    request(&saved.token, "POST", &url, Some("application/json"), Some(body.to_string().into_bytes())).await?;
    saved.link.clone().ok_or_else(|| "Drive non ha dato il link del file".to_string())
}

/// Share the saved file with the teacher at `teacher` (read-only; Drive emails them).
async fn share_with_teacher(token: &str, file_id: &str, file_name: &str, teacher: &str) -> Result<(), String> {
    let student = request(token, "GET", &format!("{DRIVE_URL}/about?fields=user(displayName)"), None, None)
        .await
        .ok()
        .and_then(|answer| drivefmt::user_name(&answer))
        .unwrap_or_default();
    let message = encode(&drivefmt::share_message(&student, file_name));
    let url = format!("{FILES_URL}/{file_id}/permissions?sendNotificationEmail=true&emailMessage={message}&fields=id");
    let body = json!({ "type": "user", "role": "reader", "emailAddress": teacher });
    request(token, "POST", &url, Some("application/json"), Some(body.to_string().into_bytes())).await.map(|_| ())
}

async fn find_or_create_folder(token: &str) -> Result<String, String> {
    let url = format!("{FILES_URL}?q={}&fields=files(id)&pageSize=1&spaces=drive", encode(&drivefmt::folder_query(FOLDER_NAME)));
    if let Some(id) = drivefmt::first_id(&request(token, "GET", &url, None, None).await?) {
        return Ok(id);
    }
    let meta = json!({ "name": FOLDER_NAME, "mimeType": drivefmt::FOLDER_MIME });
    let created = request(token, "POST", &format!("{FILES_URL}?fields=id"), Some("application/json"), Some(meta.to_string().into_bytes())).await?;
    drivefmt::created_id(&created).ok_or_else(|| "Drive non ha creato la cartella".to_string())
}

async fn find_file(token: &str, folder: &str, name: &str) -> Result<Option<String>, String> {
    let url = format!("{FILES_URL}?q={}&fields=files(id)&pageSize=1&spaces=drive", encode(&drivefmt::file_query(name, folder)));
    Ok(drivefmt::first_id(&request(token, "GET", &url, None, None).await?))
}

fn encode(s: &str) -> String {
    String::from(js_sys::encode_uri_component(s))
}

/// One authorised call to the Drive API; the answer is its JSON body.
async fn request(token: &str, method: &str, url: &str, content_type: Option<&str>, body: Option<Vec<u8>>) -> Result<Value, String> {
    let headers = js_sys::Object::new();
    set(&headers, "Authorization", &JsValue::from_str(&format!("Bearer {token}")))?;
    if let Some(content_type) = content_type {
        set(&headers, "Content-Type", &JsValue::from_str(content_type))?;
    }
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    init.set_headers(&headers);
    if let Some(body) = body {
        init.set_body(&js_sys::Uint8Array::from(body.as_slice()));
    }
    let window = web_sys::window().ok_or("nessuna finestra del browser")?;
    let response = JsFuture::from(window.fetch_with_str_and_init(url, &init))
        .await
        .map_err(js_error)?
        .dyn_into::<web_sys::Response>()
        .map_err(|_| "risposta non valida da Google".to_string())?;
    let text = JsFuture::from(response.text().map_err(js_error)?).await.map_err(js_error)?.as_string().unwrap_or_default();
    if !response.ok() {
        return Err(format!("Drive ha risposto {}: {}", response.status(), drivefmt::short(&text, ERROR_TEXT_CHARS)));
    }
    serde_json::from_str(&text).map_err(|e| format!("risposta di Drive non leggibile: {e}"))
}

/// A valid access token: the one kept from earlier, or a new one after Google's sign-in popup.
async fn access_token() -> Result<String, String> {
    let now = js_sys::Date::now();
    let kept = TOKEN.with(|t| t.borrow().as_ref().filter(|t| t.expires_at_ms - TOKEN_MARGIN_MS > now).map(|t| t.value.clone()));
    if let Some(value) = kept {
        return Ok(value);
    }
    load_identity_script().await?;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        if let Err(e) = start_token_request(&resolve, &reject) {
            let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&e));
        }
    });
    let response = JsFuture::from(promise).await.map_err(|e| e.as_string().unwrap_or_else(|| "accesso a Google non riuscito".to_string()))?;
    if let Some(error) = field(&response, "error").and_then(|e| e.as_string()) {
        return Err(format!("accesso a Google non riuscito: {error}"));
    }
    let value = field(&response, "access_token").and_then(|v| v.as_string()).ok_or("Google non ha dato il permesso di salvare")?;
    let seconds = field(&response, "expires_in").and_then(|v| v.as_f64().or_else(|| v.as_string().and_then(|s| s.parse().ok()))).unwrap_or(3000.0);
    let expires_at_ms = js_sys::Date::now() + seconds * 1000.0;
    TOKEN.with(|t| *t.borrow_mut() = Some(Token { value: value.clone(), expires_at_ms }));
    Ok(value)
}

/// Ask Google Identity Services for an access token; `resolve` gets its answer, `reject` a
/// closed or blocked popup.
fn start_token_request(resolve: &js_sys::Function, reject: &js_sys::Function) -> Result<(), String> {
    let window = web_sys::window().ok_or("nessuna finestra del browser")?;
    let oauth2 = path(window.as_ref(), &["google", "accounts", "oauth2"])?;
    let init = field(&oauth2, "initTokenClient")
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .ok_or("il login di Google non è disponibile")?;
    let on_token = {
        let resolve = resolve.clone();
        Closure::<dyn FnMut(JsValue)>::new(move |response: JsValue| {
            let _ = resolve.call1(&JsValue::NULL, &response);
        })
        .into_js_value()
    };
    let on_error = {
        let reject = reject.clone();
        Closure::<dyn FnMut(JsValue)>::new(move |error: JsValue| {
            let kind = field(&error, "type").and_then(|t| t.as_string()).unwrap_or_else(|| "errore".to_string());
            let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&format!("accesso a Google annullato ({kind})")));
        })
        .into_js_value()
    };
    let config = js_sys::Object::new();
    set(&config, "client_id", &JsValue::from_str(CLIENT_ID))?;
    set(&config, "scope", &JsValue::from_str(SCOPE))?;
    set(&config, "hd", &JsValue::from_str(DOMAIN_HINT))?;
    set(&config, "callback", &on_token)?;
    set(&config, "error_callback", &on_error)?;
    let client = init.call1(&oauth2, &config).map_err(js_error)?;
    let ask = field(&client, "requestAccessToken")
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .ok_or("il login di Google non è disponibile")?;
    ask.call0(&client).map_err(js_error)?;
    Ok(())
}

/// Load Google's sign-in script once.
async fn load_identity_script() -> Result<(), String> {
    let window = web_sys::window().ok_or("nessuna finestra del browser")?;
    if path(window.as_ref(), &["google", "accounts", "oauth2"]).is_ok() {
        return Ok(());
    }
    let document = window.document().ok_or("nessun documento del browser")?;
    let body = document.body().ok_or("la pagina non ha un corpo")?;
    let script = document.create_element("script").map_err(js_error)?;
    script.set_attribute("src", GIS_URL).map_err(js_error)?;
    script.set_attribute("async", "").map_err(js_error)?;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let on_load = Closure::once_into_js(move || {
            let _ = resolve.call0(&JsValue::NULL);
        });
        let on_error = Closure::once_into_js(move || {
            let _ = reject.call0(&JsValue::NULL);
        });
        let _ = script.add_event_listener_with_callback("load", on_load.unchecked_ref());
        let _ = script.add_event_listener_with_callback("error", on_error.unchecked_ref());
    });
    body.append_child(&script).map_err(js_error)?;
    let loaded = JsFuture::from(promise).await;
    if loaded.is_err() {
        script.remove();
        return Err("impossibile caricare il login di Google (rete bloccata?)".to_string());
    }
    Ok(())
}

/// `root.a.b.c` for `keys` = `[a, b, c]`; an error when any step is missing.
fn path(root: &JsValue, keys: &[&str]) -> Result<JsValue, String> {
    let mut current = root.clone();
    for key in keys {
        current = field(&current, key).ok_or_else(|| format!("manca {key}"))?;
    }
    Ok(current)
}

/// The property `key` of `object`, or `None` when it is missing, `undefined` or `null`.
fn field(object: &JsValue, key: &str) -> Option<JsValue> {
    js_sys::Reflect::get(object, &JsValue::from_str(key)).ok().filter(|v| !v.is_undefined() && !v.is_null())
}

fn set(object: &js_sys::Object, key: &str, value: &JsValue) -> Result<(), String> {
    js_sys::Reflect::set(object, &JsValue::from_str(key), value).map(|_| ()).map_err(js_error)
}

fn js_error(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}
