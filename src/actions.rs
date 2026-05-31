use std::path::PathBuf;
use std::fs;

// ── Context-menu items ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ContextAction {
    Open,
    OpenWith,
    Copy,
    Cut,
    Delete,
    Send,
    Paste,
    NewFolder,
}

impl ContextAction {
    pub fn label(&self) -> &'static str {
        match self {
            ContextAction::Open      => "Open",
            ContextAction::OpenWith  => "Open With...",
            ContextAction::Copy      => "Copy",
            ContextAction::Cut       => "Cut",
            ContextAction::Delete    => "Delete",
            ContextAction::Send      => "Send To...",
            ContextAction::Paste     => "Paste",
            ContextAction::NewFolder => "New Folder",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ContextAction::Open      => "->",
            ContextAction::OpenWith  => "...",
            ContextAction::Copy      => "[C]",
            ContextAction::Cut       => "[X]",
            ContextAction::Delete    => "DEL",
            ContextAction::Send      => "=>",
            ContextAction::Paste     => "[V]",
            ContextAction::NewFolder => "+DIR",
        }
    }

    /// Actions tied directly to a file/folder item
    pub fn item_actions() -> Vec<ContextAction> {
        vec![
            ContextAction::Open,
            ContextAction::OpenWith,
            ContextAction::Copy,
            ContextAction::Cut,
            ContextAction::Delete,
            ContextAction::Send,
        ]
    }

    /// Actions tied to clicking on the background empty space
    pub fn background_actions() -> Vec<ContextAction> {
        vec![
            ContextAction::NewFolder,
            ContextAction::Paste,
        ]
    }
}

// ── Context-menu state ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ContextMenu {
    pub visible: bool,
    /// Pixel position where the menu should appear.
    pub x: f32,
    pub y: f32,
    /// The file-system paths the menu applies to (selected items).
    pub targets: Vec<PathBuf>,
}

impl ContextMenu {
    pub fn open(x: f32, y: f32, targets: Vec<PathBuf>) -> Self {
        ContextMenu { visible: true, x, y, targets }
    }

    pub fn close() -> Self {
        ContextMenu::default()
    }
}

// ── Clipboard state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Clipboard {
    pub paths: Vec<PathBuf>,
    pub is_cut: bool,
}

// ── Action execution ──────────────────────────────────────────────────────────

/// Execute a context-menu action against the given targets.
/// Returns an optional error message.
pub fn execute(
    action: &ContextAction,
    targets: &[PathBuf],
    clipboard: &mut Clipboard,
) -> Option<String> {
    match action {
        ContextAction::Open => {
            for p in targets {
                if let Err(e) = open::that(p) {
                    return Some(format!("Could not open {:?}: {e}", p.file_name()));
                }
            }
            None
        }

        ContextAction::OpenWith => {
            #[cfg(target_os = "macos")]
            {
                let script = r#"
                    try
                        set appFile to choose file of type {"app"} default location (posix file "/Applications") with prompt "Select an application to open this file:"
                        return posix path of appFile
                    on error
                        return ""
                    end try
                "#;
                
                match std::process::Command::new("osascript").arg("-e").arg(script).output() {
                    Ok(output) => {
                        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !path_str.is_empty() {
                            for p in targets {
                                let _ = std::process::Command::new("open")
                                    .arg("-a")
                                    .arg(&path_str)
                                    .arg(p)
                                    .spawn();
                            }
                            None
                        } else {
                            None // Cancelled by user
                        }
                    }
                    Err(e) => Some(format!("Failed to launch app picker: {e}")),
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                for p in targets {
                    if let Err(e) = open::that(p) {
                        return Some(format!("Could not open {:?}: {e}", p.file_name()));
                    }
                }
                None
            }
        }

        ContextAction::Copy => {
            clipboard.paths = targets.to_vec();
            clipboard.is_cut = false;
            None
        }

        ContextAction::Cut => {
            clipboard.paths = targets.to_vec();
            clipboard.is_cut = true;
            None
        }

        ContextAction::Delete => {
            for p in targets {
                let result = if p.is_dir() {
                    fs::remove_dir_all(p)
                } else {
                    fs::remove_file(p)
                };
                if let Err(e) = result {
                    return Some(format!("Delete failed for {:?}: {e}", p.file_name()));
                }
            }
            None
        }

        ContextAction::Send => {
            #[cfg(target_os = "macos")]
            {
                let script = r#"
                    try
                        set targetFolder to choose folder with prompt "Select a destination folder:"
                        return posix path of targetFolder
                    on error
                        return ""
                    end try
                "#;

                match std::process::Command::new("osascript").arg("-e").arg(script).output() {
                    Ok(output) => {
                        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !path_str.is_empty() {
                            let dest_dir = std::path::PathBuf::from(path_str);
                            for p in targets {
                                if let Some(file_name) = p.file_name() {
                                    let dest = dest_dir.join(file_name);
                                    let res = if p.is_dir() {
                                        copy_dir_all(p, &dest)
                                    } else {
                                        fs::copy(p, &dest).map(|_| ())
                                    };
                                    if let Err(e) = res {
                                        return Some(format!("Send failed for {:?}: {e}", file_name));
                                    }
                                }
                            }
                            None
                        } else {
                            None // Cancelled by user
                        }
                    }
                    Err(e) => Some(format!("Failed to open destination picker: {e}")),
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                if let Some(first) = targets.first() {
                    if let Some(parent) = first.parent() {
                        let _ = open::that(parent);
                    }
                }
                None
            }
        }

        ContextAction::Paste => {
            if clipboard.paths.is_empty() {
                return Some("Clipboard is empty".to_string());
            }
            if let Some(dest_dir) = targets.first() {
                for p in &clipboard.paths {
                    if let Some(file_name) = p.file_name() {
                        let dest = dest_dir.join(file_name);
                        let res = if clipboard.is_cut {
                            fs::rename(p, &dest)
                        } else if p.is_dir() {
                            copy_dir_all(p, &dest)
                        } else {
                            fs::copy(p, &dest).map(|_| ())
                        };
                        
                        if let Err(e) = res {
                            return Some(format!("Paste failed for {:?}: {e}", file_name));
                        }
                    }
                }
                if clipboard.is_cut {
                    clipboard.paths.clear();
                }
            }
            None
        }

        ContextAction::NewFolder => {
            if let Some(dest_dir) = targets.first() {
                let mut name = "Untitled Folder".to_string();
                let mut count = 1;
                while dest_dir.join(&name).exists() {
                    count += 1;
                    name = format!("Untitled Folder {}", count);
                }
                if let Err(e) = fs::create_dir(dest_dir.join(&name)) {
                    return Some(format!("Failed to create folder: {e}"));
                }
            }
            None
        }
    }
}

fn copy_dir_all(src: impl AsRef<std::path::Path>, dst: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}