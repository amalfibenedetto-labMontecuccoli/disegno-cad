//! The classroom build's Simple mode: a short list of basic tools, menus and tool bar buttons for
//! young students, and the drafting defaults that make the screen behave like squared paper
//! (snap to the 0.5 grid, snaps on the points a student wants to hit). The Normal mode is the
//! full interface.

use cadcraft_engine::snap::mode::{CEN, END, INT, MID, PER, QUA};

/// The name of this version of the program.
pub const APP_NAME: &str = "Disegno CAD";

/// Who adapted CADCraft for the school, for the credits (empty: the school's teachers).
pub const ADAPTER: &str = "Benedetto Amalfi, docente di Tecnologia";

/// The school this version is for.
pub const SCHOOL: &str = "Scuola Secondaria di I grado R. Montecuccoli, Pavullo nel Frignano";

/// The credits line of the About window.
pub fn credits_line() -> String {
    if ADAPTER.is_empty() { format!("A cura dei docenti della {SCHOOL}.") } else { format!("A cura di {ADAPTER}, {SCHOOL}.") }
}

/// The object snaps on in Simple mode: end, middle, centre, quadrant, intersection, perpendicular.
pub const SIMPLE_OSMODE: u32 = END | MID | CEN | QUA | INT | PER;

/// The Tool Sets groups Simple mode shows.
pub const SIMPLE_GROUPS: &[&str] = &["Draw", "Hatch", "Modify", "Text", "Dimension"];

/// The Tool Sets tools Simple mode shows (command ids).
pub const SIMPLE_TOOLS: &[&str] = &[
    "line",
    "pline",
    "circle",
    "arc",
    "rectang",
    "polygon",
    "ellipse",
    "hatch",
    "move",
    "copy",
    "rotate",
    "mirror",
    "scale",
    "trim",
    "extend",
    "offset",
    "fillet",
    "arrayrect",
    "arraypolar",
    "erase",
    "text",
    "mtext",
    "dimlinear",
    "dimaligned",
    "dimradius",
    "dimdiameter",
    "dimangular",
];

/// The tool bar buttons Simple mode shows (command ids).
pub const SIMPLE_TOOLBAR: &[&str] =
    &["new", "open", "qsave", "saveas", "u", "redo", "export", "zoom.window", "pan", "zoom.extents", "ui.dialog.layers", "dist", "ui.dialog.about"];

/// The menus Simple mode shows.
pub const SIMPLE_MENUS: &[&str] = &["File", "Edit", "View", "Draw", "Modify", "Dimension", "Help"];

/// The menu items (besides the tools) Simple mode shows (command ids).
pub const SIMPLE_MENU_ITEMS: &[&str] = &[
    "new",
    "open",
    "ui.open",
    "qsave",
    "saveas",
    "ui.saveas",
    "export",
    "close",
    "ui.sendteacher",
    "u",
    "redo",
    "copyclip",
    "cutclip",
    "pasteclip",
    "selectall",
    "zoom.window",
    "zoom.extents",
    "zoom.previous",
    "pan",
    "regen",
    "ui.mode.simple",
    "ui.mode.normal",
    "ui.dialog.layers",
    "ui.dialog.language",
    "ui.dialog.about",
    "ui.dialog.commands",
    "ui.sample",
    "ui.reset",
];

/// Whether Simple mode shows the Tool Sets tool `cmd`.
pub fn tool_allowed(cmd: &str) -> bool {
    SIMPLE_TOOLS.contains(&cmd)
}

/// Whether Simple mode shows the menu entry `id`.
pub fn menu_item_allowed(id: &str) -> bool {
    SIMPLE_TOOLS.contains(&id) || SIMPLE_MENU_ITEMS.contains(&id) || id.starts_with("ui.recent.")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards the squared-paper defaults: the middle point is among the snaps (it was the one a
    /// student kept missing), and so are the end, centre and intersection ones.
    #[test]
    fn simple_snaps_include_the_midpoint() {
        for bit in [END, MID, CEN, QUA, INT, PER] {
            assert_eq!(SIMPLE_OSMODE & bit, bit);
        }
    }

    /// Guards the lists: every tool of the Tool Sets groups Simple mode shows is a real command,
    /// so a renamed command can't silently empty the student's tool palette.
    #[test]
    fn simple_tools_are_commands() {
        for tool in SIMPLE_TOOLS {
            assert!(cadcraft_engine::find_command(tool).is_some() || crate::menus::is_ui_command(tool), "{tool} is not a command");
        }
        assert!(tool_allowed("line"));
        assert!(!tool_allowed("attdef"));
        assert!(menu_item_allowed("ui.recent.0"));
        assert!(!menu_item_allowed("purge"));
    }
}
