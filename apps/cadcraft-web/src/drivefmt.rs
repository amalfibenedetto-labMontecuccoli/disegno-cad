//! The text and bytes of the Google Drive requests `drive.rs` sends, kept free of the browser so
//! they can be unit tested: query strings, the multipart upload body and reading ids out of answers.

use serde_json::Value;

/// MIME type Drive gives folders.
pub const FOLDER_MIME: &str = "application/vnd.google-apps.folder";

/// Boundary of the multipart upload body; a drawing would have to contain it by chance to break it.
const BOUNDARY: &str = "cadcraft_drive_boundary_7f3a91c2";

/// `s` as the inside of a single-quoted Drive query string.
pub fn quote(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

/// Query for the folder called `name`.
pub fn folder_query(name: &str) -> String {
    format!("name = '{}' and mimeType = '{FOLDER_MIME}' and trashed = false", quote(name))
}

/// Query for the file called `name` inside the folder `parent`.
pub fn file_query(name: &str, parent: &str) -> String {
    format!("name = '{}' and '{}' in parents and trashed = false", quote(name), quote(parent))
}

/// The `Content-Type` header and the body of a Drive multipart upload: the JSON `meta` part, then
/// the file `data` of type `mime`.
pub fn multipart(meta: &Value, mime: &str, data: &[u8]) -> (String, Vec<u8>) {
    let mut body = Vec::with_capacity(data.len().saturating_add(512));
    body.extend_from_slice(
        format!("--{BOUNDARY}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n--{BOUNDARY}\r\nContent-Type: {mime}\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--").as_bytes());
    (format!("multipart/related; boundary={BOUNDARY}"), body)
}

/// The id of the first file in a `files.list` answer.
pub fn first_id(answer: &Value) -> Option<String> {
    answer.get("files")?.get(0)?.get("id")?.as_str().map(str::to_string)
}

/// The id of a file Drive just created.
pub fn created_id(answer: &Value) -> Option<String> {
    answer.get("id")?.as_str().map(str::to_string)
}

/// Most teachers read from `docenti.json`; more are ignored.
const MAX_TEACHERS: usize = 20;

/// A teacher the drawings are sent to.
#[derive(Debug, PartialEq, Eq)]
pub struct Teacher {
    pub name: String,
    pub email: String,
}

/// The teachers listed in `docenti.json`: `{"docenti": [{"nome": "Prof. Rossi", "email": "…"}]}`.
/// Entries without something that looks like an email address are skipped.
pub fn parse_teachers(json: &Value) -> Vec<Teacher> {
    let Some(list) = json.get("docenti").and_then(Value::as_array) else { return Vec::new() };
    list.iter()
        .filter_map(|entry| {
            let email = entry.get("email")?.as_str()?.trim();
            let (user, domain) = email.split_once('@')?;
            if user.is_empty() || !domain.contains('.') || email.chars().any(char::is_whitespace) {
                return None;
            }
            let name = entry.get("nome").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).unwrap_or(email);
            Some(Teacher { name: name.to_string(), email: email.to_string() })
        })
        .take(MAX_TEACHERS)
        .collect()
}

/// The text of the email Drive sends the teacher with the shared drawing.
pub fn share_message(student: &str, file: &str) -> String {
    if student.trim().is_empty() {
        format!("Disegno inviato da un alunno: {file}")
    } else {
        format!("Disegno inviato da {student}: {file}")
    }
}

/// The signed-in user's display name in a `about` answer.
pub fn user_name(answer: &Value) -> Option<String> {
    answer.get("user")?.get("displayName")?.as_str().map(str::to_string)
}

/// At most `max` characters of `text`, for an error message.
pub fn short(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Guards the query escaping: a quote or backslash in a drawing name must not end the string
    /// early and change what the query asks for.
    #[test]
    fn queries_escape_quotes_and_backslashes() {
        assert_eq!(quote("a'b\\c"), "a\\'b\\\\c");
        assert_eq!(file_query("it's.dxf", "id1"), "name = 'it\\'s.dxf' and 'id1' in parents and trashed = false");
        assert!(folder_query("Disegni").contains("application/vnd.google-apps.folder"));
    }

    /// Guards the upload body: the metadata part comes first, the bytes follow untouched, and
    /// the closing boundary ends the body.
    #[test]
    fn multipart_wraps_the_bytes_between_metadata_and_the_closing_boundary() {
        let (content_type, body) = multipart(&json!({"name": "a.dxf"}), "application/dxf", &[1, 2, 3, 255]);
        assert!(content_type.starts_with("multipart/related; boundary="));
        let text = String::from_utf8_lossy(&body);
        assert!(text.starts_with(&format!("--{BOUNDARY}\r\nContent-Type: application/json")));
        assert!(text.contains("{\"name\":\"a.dxf\"}"));
        assert!(body.ends_with(format!("\r\n--{BOUNDARY}--").as_bytes()));
        let payload_at = body.windows(4).position(|w| w == [1, 2, 3, 255]);
        assert!(payload_at.is_some());
    }

    /// Guards reading Drive's answers: a missing or malformed answer is `None`, never a panic.
    #[test]
    fn ids_are_read_from_answers_and_bad_answers_give_none() {
        assert_eq!(first_id(&json!({"files": [{"id": "abc"}]})).as_deref(), Some("abc"));
        assert_eq!(first_id(&json!({"files": []})), None);
        assert_eq!(first_id(&json!({})), None);
        assert_eq!(first_id(&json!({"files": [{"id": 7}]})), None);
        assert_eq!(created_id(&json!({"id": "xyz"})).as_deref(), Some("xyz"));
        assert_eq!(created_id(&json!([])), None);
        assert_eq!(short("héllo wörld", 5), "héllo");
    }

    /// Guards the teacher list: good entries are kept with their names, a missing name falls
    /// back to the address, and entries without a usable address (or a broken file) are dropped
    /// rather than failing the whole send.
    #[test]
    fn teachers_are_read_from_the_config_and_bad_entries_are_dropped() {
        let config = json!({"docenti": [
            {"nome": "Prof. Rossi", "email": " rossi@smpavullo.com "},
            {"email": "bianchi@smpavullo.com"},
            {"nome": "Senza email"},
            {"nome": "Rotta", "email": "non-un-indirizzo"},
            {"nome": "Spazi", "email": "a b@x.it"},
            {"nome": "Numero", "email": 7}
        ]});
        assert_eq!(
            parse_teachers(&config),
            [
                Teacher { name: "Prof. Rossi".into(), email: "rossi@smpavullo.com".into() },
                Teacher { name: "bianchi@smpavullo.com".into(), email: "bianchi@smpavullo.com".into() },
            ]
        );
        assert!(parse_teachers(&json!({})).is_empty());
        assert!(parse_teachers(&json!({"docenti": "no"})).is_empty());
        assert!(parse_teachers(&json!([])).is_empty());
    }

    /// Guards the email text: it names the student when Google says who they are.
    #[test]
    fn share_message_names_the_student_when_known() {
        assert_eq!(share_message("Mario Rossi", "stella.dxf"), "Disegno inviato da Mario Rossi: stella.dxf");
        assert_eq!(share_message("  ", "stella.dxf"), "Disegno inviato da un alunno: stella.dxf");
        assert_eq!(user_name(&json!({"user": {"displayName": "Mario Rossi"}})).as_deref(), Some("Mario Rossi"));
        assert_eq!(user_name(&json!({})), None);
    }
}
