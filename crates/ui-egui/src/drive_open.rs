//! "Open from Drive" (classroom build): the window listing the student's drawings in the
//! "Disegni" folder of their Google Drive, saved ones and autosaved copies, to open with a click.
//! The host does the talking to Google ([`crate::Services::drive_list`] and
//! [`crate::Services::drive_open`]); the list comes back through [`CadApp::set_drive_files`].

use egui::{RichText, vec2};

use crate::theme::Tokens;
use crate::{CadApp, DriveFile};

/// The Open from Drive window.
pub fn dialog(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) {
    let t = Tokens::get();
    let mut chosen: Option<DriveFile> = None;
    let mut retry = false;
    egui::Window::new(crate::tl!("Open from Drive"))
        .id(egui::Id::new("drive_open_window"))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_width(440.0);
            ui.set_max_width(520.0);
            match &app.ui.drive_listing {
                None => {
                    ui.label(crate::tl!("Looking for your drawings in Google Drive..."));
                }
                Some(Err(e)) => {
                    ui.colored_label(egui::Color32::from_rgb(230, 80, 70), e);
                    if ui.button(crate::tl!("Try again")).clicked() {
                        retry = true;
                    }
                }
                Some(Ok(files)) if files.is_empty() => {
                    ui.label(crate::tl!("There are no drawings in your Drive yet. Save or hand in a drawing first."));
                }
                Some(Ok(files)) => {
                    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                        for file in files {
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(RichText::new(crate::tl!("Open")).size(14.0)).min_size(vec2(64.0, 26.0))).clicked() {
                                    chosen = Some(file.clone());
                                }
                                ui.label(RichText::new(&file.name).size(15.0));
                                ui.label(RichText::new(&file.modified).small().color(t.text_dim));
                            });
                        }
                    });
                }
            }
        });
    if retry {
        app.request_drive_list();
    }
    if let Some(file) = chosen {
        *open = false;
        app.open_from_drive(&file);
    }
}
