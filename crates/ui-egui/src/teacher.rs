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

/// The hand-in window. The main way is "Save and copy link": the drawing goes to the student's
/// Drive and they paste its link into the Classroom assignment. Below it, the other way: send the
/// file by email to a teacher, whose address the student writes (Send runs `ui.sendteacher`).
pub fn dialog(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) {
    let domain = app.services.teacher_domain.clone();
    let (mut send, mut link, mut cancel) = (false, false, false);
    egui::Window::new(crate::tl!("Hand in your drawing"))
        .id(egui::Id::new("send_to_teacher_window"))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.label(crate::tl!("Save the drawing in your Google Drive and get the link to paste into the Classroom assignment."));
            ui.add_space(8.0);
            if ui.add(egui::Button::new(egui::RichText::new(crate::tl!("Save and copy link")).size(16.0)).min_size(vec2(380.0, 36.0))).clicked() {
                link = true;
            }
            ui.add_space(10.0);
            ui.separator();
            ui.label(crate::tl!("Or send it by email to a teacher:"));
            let hint = match &domain {
                Some(domain) => format!("name@{domain}"),
                None => "name@school.edu".to_string(),
            };
            let field = ui.add(egui::TextEdit::singleline(&mut app.ui.teacher_email).hint_text(hint).desired_width(380.0));
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
    if link {
        app.ui.send_error.clear();
        *open = false;
        if let Err(e) = app.run("ui.sharelink", json!({})) {
            app.set_status(e);
        }
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

/// The window shown once the drawing is saved and linked: the link, a Copy button and what to do
/// with it in Classroom.
pub fn link_dialog(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) {
    let Some((name, link)) = app.ui.share_link.clone() else {
        *open = false;
        return;
    };
    let copied_id = egui::Id::new(("share_link_copied", &link));
    let copied = ctx.data(|d| d.get_temp::<bool>(copied_id)).unwrap_or(false);
    let mut copy = false;
    egui::Window::new(crate::tl!("Drawing saved"))
        .id(egui::Id::new("share_link_window"))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.label(format!("{} {name}", crate::tl!("Saved in your Google Drive:")));
            ui.add_space(6.0);
            ui.label(crate::tl!("Link to paste into Classroom:"));
            // A copy to select from: edits to it are thrown away.
            let mut shown = link.clone();
            ui.add(egui::TextEdit::singleline(&mut shown).desired_width(420.0));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(egui::RichText::new(crate::tl!("Copy link")).size(15.0)).min_size(vec2(140.0, 30.0))).clicked() {
                    copy = true;
                }
                if copied {
                    ui.colored_label(egui::Color32::from_rgb(90, 190, 110), crate::tl!("Link copied"));
                }
            });
            ui.add_space(6.0);
            ui.label(crate::tl!("In Classroom: open the assignment, choose Add or create, then Link, and paste it."));
        });
    if copy {
        ctx.copy_text(link);
        ctx.data_mut(|d| d.insert_temp(copied_id, true));
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
