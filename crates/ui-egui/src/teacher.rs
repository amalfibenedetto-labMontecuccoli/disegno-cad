//! "Send to teacher" (classroom build): the window that asks the student for their teacher's school
//! email address, and the check that address must pass. The send itself is the `ui.sendteacher`
//! command (`menus.rs`), which saves the drawing and tells the host to pass it on to that address.

use egui::vec2;
use serde_json::json;

use crate::CadApp;

/// The address in `input` lower-cased and trimmed, or why it can't be used. With `domain` set it
/// must be an address of that domain, so a drawing never leaves the school by a typo.
pub fn check(input: &str, domain: Option<&str>) -> Result<String, String> {
    let email = input.trim().to_lowercase();
    if email.is_empty() {
        return Err(crate::tl!("Write your teacher's email address.").to_string());
    }
    let Some((user, host)) = email.split_once('@') else {
        return Err(crate::tl!("The email address must contain @.").to_string());
    };
    if user.is_empty() || !host.contains('.') || host.starts_with('.') || host.ends_with('.') || email.chars().any(char::is_whitespace) {
        return Err(crate::tl!("This email address is not valid.").to_string());
    }
    if let Some(domain) = domain
        && !host.eq_ignore_ascii_case(domain)
    {
        return Err(format!("{} @{domain}", crate::tl!("Use your teacher's school address, ending with")));
    }
    Ok(email)
}

/// The window asking for the teacher's address; Send runs `ui.sendteacher` with it.
pub fn dialog(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) {
    let domain = app.services.teacher_domain.clone();
    let (mut send, mut cancel) = (false, false);
    egui::Window::new(crate::tl!("Send to teacher"))
        .id(egui::Id::new("send_to_teacher_window"))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label(crate::tl!("Your drawing is saved in your Google Drive and shared with your teacher."));
            ui.add_space(6.0);
            ui.label(crate::tl!("Teacher's email address"));
            let hint = match &domain {
                Some(domain) => format!("name@{domain}"),
                None => "name@school.edu".to_string(),
            };
            let field = ui.add(egui::TextEdit::singleline(&mut app.ui.teacher_email).hint_text(hint).desired_width(320.0));
            if !field.has_focus() && app.ui.send_error.is_empty() {
                field.request_focus();
            }
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                send = true;
            }
            if !app.ui.send_error.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(230, 80, 70), &app.ui.send_error);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(crate::tl!("Send")).clicked() {
                    send = true;
                }
                if ui.button(crate::tl!("Cancel")).clicked() {
                    cancel = true;
                }
            });
        });
    if cancel {
        app.ui.send_error.clear();
        *open = false;
    }
    if send {
        match check(&app.ui.teacher_email, domain.as_deref()) {
            Ok(email) => {
                app.ui.send_error.clear();
                *open = false;
                if let Err(e) = app.run("ui.sendteacher", json!({ "email": email })) {
                    app.set_status(e);
                }
            }
            Err(e) => app.ui.send_error = e,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::check;

    /// Guards the address check: it trims and lower-cases a good address, and refuses an empty
    /// one, one with no @ or no dot in its domain, spaces, and (when a school domain is set) any
    /// other domain, so a drawing never goes to a mistyped or outside address.
    #[test]
    fn teacher_address_is_checked_and_kept_inside_the_school_domain() {
        assert_eq!(check("  Rossi@SMPavullo.com ", Some("smpavullo.com")).as_deref(), Ok("rossi@smpavullo.com"));
        assert_eq!(check("rossi@gmail.com", None).as_deref(), Ok("rossi@gmail.com"));
        assert!(check("", None).is_err());
        assert!(check("rossi", None).is_err());
        assert!(check("@smpavullo.com", None).is_err());
        assert!(check("rossi@smpavullo", None).is_err());
        assert!(check("ro ssi@smpavullo.com", None).is_err());
        assert!(check("rossi@gmail.com", Some("smpavullo.com")).is_err());
        assert!(check("rossi@evil.smpavullo.com", Some("smpavullo.com")).is_err());
    }
}
