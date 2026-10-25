//! "Send to teacher" (classroom build): the window that asks the student for their teacher's school
//! email address, and the check that address must pass. The send itself is the `ui.sendteacher`
//! command (`menus.rs`), which saves the drawing and tells the host to pass it on to that address.

use egui::{Rect, RichText, vec2};
use serde_json::json;

use crate::CadApp;
use crate::theme::Tokens;

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

/// A file name typed by a student, without folders, odd characters or extension.
pub fn file_stem(text: &str) -> String {
    let cleaned: String = text.chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') && !c.is_control()).collect();
    let cleaned = cleaned.trim();
    let stem = match cleaned.rsplit_once('.') {
        Some((stem, ext)) if ext.eq_ignore_ascii_case("dxf") || ext.eq_ignore_ascii_case("dwg") => stem,
        _ => cleaned,
    };
    stem.trim().chars().take(80).collect()
}

/// The "Save to Drive" window: the student names the file, then it goes to their Drive.
pub fn save_dialog(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) {
    let (mut save, mut cancel) = (false, false);
    egui::Window::new(crate::tl!("Save to Drive"))
        .id(egui::Id::new("save_to_drive_window"))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.label(crate::tl!("Name of the file:"));
            let field = ui.add(egui::TextEdit::singleline(&mut app.ui.save_name).desired_width(380.0));
            if !field.has_focus() && !field.lost_focus() {
                field.request_focus();
            }
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                save = true;
            }
            if !app.ui.send_error.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(230, 80, 70), crate::i18n::t(&app.ui.send_error).to_string());
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(egui::RichText::new(crate::tl!("Save")).size(15.0)).min_size(vec2(120.0, 30.0))).clicked() {
                    save = true;
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
    if save {
        let name = app.ui.save_name.clone();
        match app.run("ui.drive.save", json!({ "name": name })) {
            Ok(_) => {
                app.ui.send_error.clear();
                *open = false;
            }
            Err(e) => app.ui.send_error = e,
        }
    }
}

/// The window shown once the drawing is saved and linked: the link, a Copy button and what to do
/// with it in Classroom.
pub fn link_dialog(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) {
    let Some((name, link, folder)) = app.ui.share_link.clone() else {
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
            ui.add_space(6.0);
            ui.hyperlink_to(crate::tl!("Open the Disegni folder in Drive"), &folder);
        });
    if copy {
        ctx.copy_text(link);
        ctx.data_mut(|d| d.insert_temp(copied_id, true));
    }
}

/// The classroom build's sign-in cover: until a student signs in with the school Google account,
/// a full-screen cover with one big button sits over the whole window and swallows every click
/// and key, so nobody starts a drawing that could be lost for want of a place to save it.
pub fn signin_gate(app: &mut CadApp, ctx: &egui::Context) {
    let t = Tokens::get();
    let screen = ctx.content_rect();
    let domain = app.services.teacher_domain.clone().unwrap_or_else(|| "school.edu".to_string());
    let mut go = false;
    egui::Area::new(egui::Id::new("sign_in_gate")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
        // The whole window is one widget, so no pointer event reaches what is underneath.
        ui.allocate_rect(screen, egui::Sense::click_and_drag());
        ui.painter().rect_filled(screen, 0.0, egui::Color32::from_black_alpha(225));
        let card = Rect::from_center_size(screen.center(), vec2((screen.width() - 40.0).clamp(280.0, 620.0), 380.0));
        ui.painter().rect_filled(card, 14.0, t.panel);
        ui.painter().rect_stroke(card, 14.0, egui::Stroke::new(1.5, t.border), egui::StrokeKind::Inside);
        ui.scope_builder(egui::UiBuilder::new().max_rect(card.shrink(30.0)), |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(crate::tl!("Welcome")).size(34.0).strong());
                ui.add_space(12.0);
                ui.label(RichText::new(crate::tl!("Sign in with your school Google account to start drawing. Your drawings are saved in your Google Drive.")).size(19.0));
                ui.add_space(22.0);
                let button = egui::Button::new(RichText::new(crate::tl!("Sign in with Google")).size(24.0)).min_size(vec2(card.width() - 60.0, 70.0));
                if ui.add(button).clicked() {
                    go = true;
                }
                ui.add_space(14.0);
                ui.label(RichText::new(format!("{} name@{domain}", crate::tl!("A Google window will open: choose your school account,"))).size(15.0).color(t.text_dim));
            });
        });
    });
    if go {
        app.start("ui.signin");
    }
}

/// The window offered when the app opens: sign in with the school's Google account once, so
/// saving and handing in don't ask again. It closes itself when the sign-in is done.
pub fn signin_dialog(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) {
    if app.services.account.as_ref().and_then(|name| name()).is_some() {
        *open = false;
        return;
    }
    let (mut go, mut later) = (false, false);
    egui::Window::new(crate::tl!("Welcome"))
        .id(egui::Id::new("sign_in_window"))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.label(crate::tl!("Sign in with your school Google account to save your drawings and hand them in."));
            ui.add_space(8.0);
            if ui.add(egui::Button::new(egui::RichText::new(crate::tl!("Sign in with Google")).size(16.0)).min_size(vec2(380.0, 36.0))).clicked() {
                go = true;
            }
            ui.add_space(4.0);
            if ui.button(crate::tl!("Later")).clicked() {
                later = true;
            }
        });
    if go {
        *open = false;
        app.start("ui.signin");
    }
    if later {
        *open = false;
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
