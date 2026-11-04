//! Resolve a selected file before creating its parent-directory workspace.
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileWorkspace {
    pub session_id: String,
    pub directory: String,
    pub path: String,
}
pub(crate) struct Target {
    pub directory: String,
    pub path: String,
}
pub(crate) fn resolve(file: &str) -> Result<Target, String> {
    if !Path::new(file).is_absolute() {
        return Err("workspace file path must be absolute".into());
    }
    let file = std::fs::canonicalize(file).map_err(|error| error.to_string())?;
    if !file.is_file() {
        return Err("workspace file is not a file".into());
    }
    let parent = file.parent().ok_or("workspace file has no parent")?;
    Ok(Target {
        directory: parent
            .to_str()
            .ok_or("workspace directory is not valid Unicode")?
            .to_owned(),
        path: file
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("workspace filename is not valid Unicode")?
            .to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_file_opens_its_canonical_parent_and_exact_unicode_name() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("界 🌍.txt");
        std::fs::write(&file, "untouched").unwrap();
        let target = resolve(file.to_str().unwrap()).unwrap();
        assert_eq!(
            target.directory,
            temp.path().canonicalize().unwrap().to_str().unwrap()
        );
        assert_eq!(target.path, "界 🌍.txt");
        assert_eq!(std::fs::read_to_string(file).unwrap(), "untouched");
        assert!(resolve(temp.path().to_str().unwrap()).is_err());
        assert!(resolve("relative.txt").is_err());
        assert!(resolve(temp.path().join("missing.txt").to_str().unwrap()).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn selected_symlink_uses_the_real_file_parent_without_moving_or_copying_it() {
        let temp = tempfile::tempdir().unwrap();
        let real = tempfile::tempdir().unwrap();
        let file = real.path().join("actual.txt");
        std::fs::write(&file, "original").unwrap();
        let link = temp.path().join("alias.txt");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        let target = resolve(link.to_str().unwrap()).unwrap();
        assert_eq!(
            target.directory,
            real.path().canonicalize().unwrap().to_str().unwrap()
        );
        assert_eq!(target.path, "actual.txt");
        assert!(link.is_symlink());
        assert_eq!(std::fs::read_to_string(file).unwrap(), "original");
    }
}
