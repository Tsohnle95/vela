//! Session window dimensions. Position and maximized/fullscreen geometry are
//! deliberately excluded, matching Vela's window-bounds.json contract.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, io, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: f64,
    pub height: f64,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 1280.,
            height: 800.,
        }
    }
}

impl WindowSize {
    pub fn new(width: f64, height: f64) -> Option<Self> {
        (width.is_finite() && height.is_finite() && width >= 200. && height >= 200.)
            .then_some(Self { width, height })
    }

    pub fn parse(text: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(text).ok()?;
        let parse = |value: &Value| {
            Self::new(
                value.get("width")?.as_f64()?,
                value.get("height")?.as_f64()?,
            )
        };
        parse(&value).or_else(|| {
            (value.get("version")?.as_u64()? == 2)
                .then(|| parse(value.get("session")?))
                .flatten()
        })
    }

    pub fn load(directory: &Path) -> Option<Self> {
        Self::parse(&fs::read_to_string(directory.join("window-bounds.json")).ok()?)
    }

    fn encode(self) -> io::Result<Vec<u8>> {
        if Self::new(self.width, self.height).is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid session window size",
            ));
        }
        serde_json::to_vec(&serde_json::json!({"version": 2, "session": self}))
            .map_err(io::Error::other)
    }

    /// Called by the engine's serialized command loop, away from rendering.
    pub fn save(self, directory: &Path) -> io::Result<()> {
        let bytes = self.encode()?;
        fs::create_dir_all(directory)?;
        let temporary = directory.join("window-bounds.json.tmp");
        fs::write(&temporary, bytes)?;
        fs::rename(temporary, directory.join("window-bounds.json"))
    }

    /// A legacy profile supplies the initial dimensions only. Never replace an
    /// existing native file, including an unreadable or future-format file.
    pub fn import_if_absent(self, directory: &Path) -> io::Result<()> {
        use io::Write;
        let bytes = self.encode()?;
        fs::create_dir_all(directory)?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("window-bounds.json"))
        {
            Ok(mut file) => file.write_all(&bytes),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_reference_formats_without_clamping_small_windows() {
        assert_eq!(
            WindowSize::parse(r#"{"width":250,"height":210}"#),
            WindowSize::new(250., 210.)
        );
        assert_eq!(
            WindowSize::parse(r#"{"version":2,"session":{"width":500,"height":350}}"#),
            WindowSize::new(500., 350.)
        );
        for invalid in [
            r#"{"version":3,"session":{"width":500,"height":350}}"#,
            r#"{"width":199,"height":800}"#,
            r#"{"version":2,"session":{"width":"500","height":350}}"#,
            "null",
            "bad json",
        ] {
            assert!(WindowSize::parse(invalid).is_none(), "{invalid}");
        }
        assert!(WindowSize::new(f64::INFINITY, 800.).is_none());
        assert!(WindowSize::new(800., f64::NAN).is_none());
    }

    #[test]
    fn legacy_import_never_overwrites_native_dimensions_and_resizes_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let initial = WindowSize::new(640., 480.).unwrap();
        initial.import_if_absent(temp.path()).unwrap();
        WindowSize::default().import_if_absent(temp.path()).unwrap();
        assert_eq!(WindowSize::load(temp.path()), Some(initial));
        let resized = WindowSize::new(1000., 700.).unwrap();
        resized.save(temp.path()).unwrap();
        assert_eq!(WindowSize::load(temp.path()), Some(resized));
        let file = temp.path().join("window-bounds.json");
        fs::write(&file, r#"{"version":99}"#).unwrap();
        initial.import_if_absent(temp.path()).unwrap();
        assert_eq!(fs::read_to_string(file).unwrap(), r#"{"version":99}"#);
    }
}
