use std::path::PathBuf;
use std::fs;
use file_icon_provider::get_file_icon;
use crate::ICON_SIZE;

// ── Entry ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub icon: Option<iced::widget::image::Handle>,
}

// ── Volume discovery ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Volume {
    pub label: String,
    pub path: PathBuf,
    pub icon: Option<iced::widget::image::Handle>,
}

/// Returns mounted volumes / bookmarks for the sidebar.
pub fn get_volumes() -> Vec<Volume> {
    let mut vols = Vec::new();

    let mut add_vol = |label: String, path: PathBuf| {
        let icon = get_file_icon(&path, 24)
            .ok()
            .map(|ic| iced::widget::image::Handle::from_rgba(
                ic.width, ic.height, ic.pixels,
            ));
        vols.push(Volume { label, path, icon });
    };

    // Home directory
    if let Some(home) = dirs::home_dir() {
        add_vol("Home".into(), home.clone());

        // Common XDG-style subdirs
        for (label, sub) in &[
            ("Desktop",   "Desktop"),
            ("Documents", "Documents"),
            ("Downloads", "Downloads"),
            ("Music",     "Music"),
            ("Pictures",  "Pictures"),
            ("Videos",    "Videos"),
        ] {
            let p = home.join(sub);
            if p.exists() {
                add_vol((*label).into(), p);
            }
        }
    }

    // Root filesystem
    add_vol("/ (root)".into(), PathBuf::from("/"));

    // Linux: scan /media and /mnt for mounted drives
    #[cfg(target_os = "linux")]
    {
        for base in &["/media", "/mnt"] {
            if let Ok(rd) = fs::read_dir(base) {
                for entry in rd.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        let label = entry.file_name().to_string_lossy().into_owned();
                        // Also scan user sub-dirs under /media/<user>/
                        if let Ok(sub) = fs::read_dir(&path) {
                            let mut any = false;
                            for se in sub.filter_map(|e| e.ok()) {
                                let sp = se.path();
                                if sp.is_dir() {
                                    add_vol(format!("{}/{}", label, se.file_name().to_string_lossy()), sp);
                                    any = true;
                                }
                            }
                            if !any {
                                add_vol(label, path);
                            }
                        } else {
                            add_vol(label, path);
                        }
                    }
                }
            }
        }
    }

    // macOS: scan /Volumes
    #[cfg(target_os = "macos")]
    {
        if let Ok(rd) = fs::read_dir("/Volumes") {
            for entry in rd.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let label = entry.file_name().to_string_lossy().into_owned();
                    add_vol(label, path);
                }
            }
        }
    }

    vols
}

// ── Directory loading ─────────────────────────────────────────────────────────

pub fn load_entries(path: &PathBuf) -> Result<Vec<Entry>, String> {
    match fs::read_dir(path) {
        Ok(rd) => {
            let mut entries: Vec<Entry> = rd
                .filter_map(|e| e.ok())
                .map(|e| {
                    let path = e.path();
                    let name = e.file_name().to_string_lossy().to_string();
                    let is_dir = path.is_dir();
                    let icon = get_file_icon(&path, ICON_SIZE)
                        .ok()
                        .map(|ic| iced::widget::image::Handle::from_rgba(
                            ic.width, ic.height, ic.pixels,
                        ));
                    Entry { name, path, is_dir, icon }
                })
                .filter(|e| !e.name.starts_with('.'))
                .collect();

            entries.sort_by(|a, b| {
                b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
            });

            Ok(entries)
        }
        Err(e) => Err(e.to_string()),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max - 1).collect();
        format!("{}…", t)
    }
}

pub fn ancestor_paths(current: &PathBuf) -> Vec<PathBuf> {
    let mut components: Vec<PathBuf> = Vec::new();
    let mut p = current.clone();
    loop {
        components.push(p.clone());
        match p.parent() {
            Some(parent) if parent != p => p = parent.to_path_buf(),
            _ => break,
        }
    }
    components.reverse();
    components
}