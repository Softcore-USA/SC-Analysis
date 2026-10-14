use egui::{CollapsingHeader, RichText, ScrollArea, Ui};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

/// File extensions the navigator lists as trace files.
pub const TRACE_EXTENSIONS: [&str; 3] = ["bin", "csv", "txt"];

/// Something the user asked for in the navigator.
pub enum WorkspaceAction {
    /// Add these files to the active trace plotter (or a new one if none is open)
    AddToPlotter(Vec<PathBuf>),
    /// Open these files in a new trace plotter window
    OpenInNewPlotter(Vec<PathBuf>),
    /// Remove these files' traces from the active trace plotter
    RemoveFromPlotter(Vec<PathBuf>),
}

/// A directory opened as a workspace, shown as a file tree.
pub struct Workspace {
    root: DirNode,
}

struct DirNode {
    path: PathBuf,
    name: String,
    /// Loaded the first time the folder is expanded
    children: Option<DirContents>,
}

struct DirContents {
    dirs: Vec<DirNode>,
    files: Vec<PathBuf>,
    error: Option<String>,
}

impl Workspace {
    pub fn open(root: PathBuf) -> Self {
        let name = display_name(&root);
        Workspace {
            root: DirNode {
                path: root,
                name,
                children: None,
            },
        }
    }

    pub fn root(&self) -> &Path {
        &self.root.path
    }

    pub fn name(&self) -> &str {
        &self.root.name
    }

    /// Forgets the cached folder contents so the tree is re-read from disk.
    pub fn refresh(&mut self) {
        self.root.children = None;
    }

    /// Path relative to the workspace root, used as a readable trace name.
    pub fn relative_name(&self, path: &Path) -> String {
        path.strip_prefix(&self.root.path)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    /// Draws the file tree. `is_loaded` tells whether a file is already in the
    /// active plotter so it can be highlighted.
    pub fn render(&mut self, ui: &mut Ui, is_loaded: &dyn Fn(&Path) -> bool) -> Option<WorkspaceAction> {
        let mut action = None;
        ScrollArea::both()
            .id_source("workspace_tree")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // The root folder's contents are always shown, without a header
                render_dir_contents(&mut self.root, ui, is_loaded, &mut action);
            });
        action
    }
}

fn render_dir_contents(
    node: &mut DirNode,
    ui: &mut Ui,
    is_loaded: &dyn Fn(&Path) -> bool,
    action: &mut Option<WorkspaceAction>,
) {
    let contents = node.children.get_or_insert_with(|| read_dir(&node.path));

    if let Some(error) = &contents.error {
        ui.colored_label(ui.visuals().warn_fg_color, error);
    }

    for dir in &mut contents.dirs {
        let header = CollapsingHeader::new(&dir.name)
            .id_source(&dir.path)
            .show(ui, |ui| {
                render_dir_contents(dir, ui, is_loaded, action);
            });

        header.header_response.context_menu(|ui| {
            // Reuses the cached listing (reads the folder once if it was never expanded)
            let files = &dir.children.get_or_insert_with(|| read_dir(&dir.path)).files;
            let enabled = !files.is_empty();
            if ui
                .add_enabled(
                    enabled,
                    egui::Button::new(format!("Add all traces in folder ({})", files.len())),
                )
                .clicked()
            {
                *action = Some(WorkspaceAction::AddToPlotter(files.clone()));
                ui.close_menu();
            }
            if ui
                .add_enabled(enabled, egui::Button::new("Open folder in new plotter"))
                .clicked()
            {
                *action = Some(WorkspaceAction::OpenInNewPlotter(files.clone()));
                ui.close_menu();
            }
        });
    }

    for file in &contents.files {
        let name = display_name(file);
        let loaded = is_loaded(file);
        let text = if loaded {
            RichText::new(name).strong()
        } else {
            RichText::new(name)
        };

        let response = ui
            .selectable_label(loaded, text)
            .on_hover_text(if loaded {
                "In the active plotter. Click to remove it."
            } else {
                "Click to add to the active plotter"
            });

        // Clicking toggles the file in / out of the active plotter
        if response.clicked() {
            *action = Some(if loaded {
                WorkspaceAction::RemoveFromPlotter(vec![file.clone()])
            } else {
                WorkspaceAction::AddToPlotter(vec![file.clone()])
            });
        }

        response.context_menu(|ui| {
            if loaded {
                if ui.button("Remove from active plotter").clicked() {
                    *action = Some(WorkspaceAction::RemoveFromPlotter(vec![file.clone()]));
                    ui.close_menu();
                }
            } else if ui.button("Add to active plotter").clicked() {
                *action = Some(WorkspaceAction::AddToPlotter(vec![file.clone()]));
                ui.close_menu();
            }
            if ui.button("Open in new plotter").clicked() {
                *action = Some(WorkspaceAction::OpenInNewPlotter(vec![file.clone()]));
                ui.close_menu();
            }
        });
    }

    if contents.dirs.is_empty() && contents.files.is_empty() && contents.error.is_none() {
        ui.weak("(no trace files)");
    }
}

fn read_dir(path: &Path) -> DirContents {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            return DirContents {
                dirs,
                files,
                error: Some(format!("Could not read folder: {}", e)),
            }
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = display_name(&path);
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            dirs.push(DirNode {
                path,
                name,
                children: None,
            });
        } else if is_trace_file(&path) {
            files.push(path);
        }
    }

    dirs.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    files.sort_by(|a, b| natural_cmp(&display_name(a), &display_name(b)));

    DirContents {
        dirs,
        files,
        error: None,
    }
}

pub fn is_trace_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| TRACE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Compares names so that "trace2" sorts before "trace10".
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) if ca.is_ascii_digit() && cb.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = a_chars.peek().filter(|c| c.is_ascii_digit()) {
                    na.push(*c);
                    a_chars.next();
                }
                let mut nb = String::new();
                while let Some(c) = b_chars.peek().filter(|c| c.is_ascii_digit()) {
                    nb.push(*c);
                    b_chars.next();
                }
                let na_trim = na.trim_start_matches('0');
                let nb_trim = nb.trim_start_matches('0');
                let ord = na_trim
                    .len()
                    .cmp(&nb_trim.len())
                    .then_with(|| na_trim.cmp(nb_trim));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(ca), Some(cb)) => {
                let ord = ca.to_ascii_lowercase().cmp(&cb.to_ascii_lowercase());
                if ord != Ordering::Equal {
                    return ord;
                }
                a_chars.next();
                b_chars.next();
            }
        }
    }
}
