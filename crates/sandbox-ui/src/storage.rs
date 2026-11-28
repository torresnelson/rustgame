//! Small text saves (the tetris high score table): localStorage in the
//! browser, a file in the user's data folder on desktop.

/// The saved text for `key`, if any.
pub fn load_text(key: &str) -> Option<String> {
    platform::load_text(key)
}

/// Save `value` under `key`; failures are reported, not fatal.
pub fn save_text(key: &str, value: &str) -> Result<(), String> {
    platform::save_text(key, value)
}

/// A best score saved as plain text; missing or unreadable is 0.
pub fn parse_best(text: Option<&str>) -> u32 {
    text.and_then(|text| text.trim().parse().ok()).unwrap_or(0)
}

/// The best score saved under `key`.
pub fn load_best(key: &str) -> u32 {
    parse_best(load_text(key).as_deref())
}

/// Save a best score under `key`; failures are reported, not fatal.
pub fn save_best(key: &str, best: u32) -> Result<(), String> {
    save_text(key, &format!("{best}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_best_falls_back_to_zero() {
        assert_eq!(parse_best(None), 0);
        assert_eq!(parse_best(Some("42\n")), 42);
        assert_eq!(parse_best(Some("lots")), 0);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::path::{Path, PathBuf};

    /// Folder name of the saves; earlier builds used `LEGACY_NAME`.
    const NAME: &str = "rustgame";
    const LEGACY_NAME: &str = "pygame-sandbox";

    /// `$XDG_DATA_HOME`, `~/.local/share` or `%APPDATA%`.
    fn data_base() -> PathBuf {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Where saves are written.
    pub fn data_dir() -> PathBuf {
        data_base().join(NAME)
    }

    pub fn load_text_in(dir: &Path, key: &str) -> Option<String> {
        std::fs::read_to_string(dir.join(key)).ok()
    }

    /// The first of `dirs` that has `key`: saves from before the folder was
    /// renamed still load, and the next save writes them to the new one.
    pub fn load_text_from(dirs: &[PathBuf], key: &str) -> Option<String> {
        dirs.iter().find_map(|dir| load_text_in(dir, key))
    }

    pub fn save_text_in(dir: &Path, key: &str, value: &str) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        std::fs::write(dir.join(key), value).map_err(|error| error.to_string())
    }

    pub fn load_text(key: &str) -> Option<String> {
        load_text_from(&[data_dir(), data_base().join(LEGACY_NAME)], key)
    }

    pub fn save_text(key: &str, value: &str) -> Result<(), String> {
        save_text_in(&data_dir(), key, value)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn save_then_load_roundtrip_creates_the_folder() {
            let dir = std::env::temp_dir()
                .join(format!("sandbox-ui-storage-{}", std::process::id()))
                .join("nested");
            let _ = std::fs::remove_dir_all(&dir);
            assert_eq!(load_text_in(&dir, "tetris.highscores"), None);
            save_text_in(&dir, "tetris.highscores", "[]\n").unwrap();
            assert_eq!(
                load_text_in(&dir, "tetris.highscores").as_deref(),
                Some("[]\n")
            );
            let _ = std::fs::remove_dir_all(dir.parent().unwrap());
        }

        #[test]
        fn saves_in_the_old_folder_still_load() {
            let base =
                std::env::temp_dir().join(format!("sandbox-ui-legacy-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&base);
            let (new, old) = (base.join(NAME), base.join(LEGACY_NAME));
            save_text_in(&old, "snake.best", "40\n").unwrap();
            let dirs = [new.clone(), old];
            assert_eq!(load_text_from(&dirs, "snake.best").as_deref(), Some("40\n"));
            // Once saved in the new folder, that copy wins.
            save_text_in(&new, "snake.best", "90\n").unwrap();
            assert_eq!(load_text_from(&dirs, "snake.best").as_deref(), Some("90\n"));
            assert_eq!(load_text_from(&dirs, "missing"), None);
            let _ = std::fs::remove_dir_all(base);
        }

        #[test]
        fn data_dir_is_named_after_the_project() {
            assert!(data_dir().ends_with("rustgame"));
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    pub fn load_text(key: &str) -> Option<String> {
        storage()?.get_item(key).ok().flatten()
    }

    pub fn save_text(key: &str, value: &str) -> Result<(), String> {
        let storage = storage().ok_or("localStorage is unavailable")?;
        storage
            .set_item(key, value)
            .map_err(|error| format!("{error:?}"))
    }
}
