//! Source inventory includes partials as dependencies, never as CSS outputs.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(PartialEq, Eq)]
pub(crate) struct Inventory {
    versions: BTreeMap<PathBuf, (SystemTime, u64)>,
    directories: BTreeSet<PathBuf>,
    pub entries: Vec<PathBuf>,
}
fn ignored(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | ".git"
            | "dist"
            | "build"
            | ".next"
            | ".cache"
            | ".vela-recovery"
            | ".mira-recovery"
            | ".openshell-recovery"
            | ".orbit-recovery"
            | "coverage"
            | "out"
            | "vendor"
    )
}
fn source(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("scss") || extension.eq_ignore_ascii_case("sass")
    })
}
pub(crate) fn configuration(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).is_ok_and(|relative| {
        [
            "vela-sass.json",
            "mira-sass.json",
            "orbit-sass.json",
            "package.json",
            ".vscode/settings.json",
        ]
        .iter()
        .any(|name| relative == Path::new(name))
    })
}
pub(crate) fn ignored_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).map_or(true, |relative| {
        relative.components().any(|part| {
            !matches!(part, std::path::Component::Normal(_))
                || ignored(&part.as_os_str().to_string_lossy())
        })
    })
}
pub(crate) fn relevant(root: &Path, path: &Path) -> bool {
    !ignored_path(root, path) && (source(path) || configuration(root, path))
}
pub(crate) fn scan(root: &Path) -> Result<Inventory, String> {
    let metadata = fs::symlink_metadata(root).map_err(|error| error.to_string())?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || fs::canonicalize(root).map_err(|error| error.to_string())? != root
    {
        return Err("Sass workspace root changed; reopen it before compiling".into());
    }
    let mut inventory = Inventory {
        versions: BTreeMap::new(),
        directories: BTreeSet::new(),
        entries: Vec::new(),
    };
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let name = entry.file_name();
            if ignored(&name.to_string_lossy()) {
                continue;
            }
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            let path = entry.path();
            if kind.is_dir() {
                inventory.directories.insert(path.clone());
                pending.push(path);
            } else if kind.is_file() && (source(&path) || configuration(root, &path)) {
                let metadata = entry.metadata().map_err(|error| error.to_string())?;
                inventory.versions.insert(
                    path.clone(),
                    (
                        metadata.modified().map_err(|error| error.to_string())?,
                        metadata.len(),
                    ),
                );
                if source(&path) && !name.to_string_lossy().starts_with('_') {
                    inventory.entries.push(path);
                }
            }
        }
    }
    inventory.entries.sort();
    Ok(inventory)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_add_edit_and_delete_change_inventory_without_becoming_outputs() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/main.SCSS"), ".a { color: red }").unwrap();
        let first = scan(&root).unwrap();
        assert_eq!(first.entries, [root.join("nested/main.SCSS")]);
        fs::write(root.join("nested/_ink.scss"), "$ink: red").unwrap();
        let added = scan(&root).unwrap();
        assert!(first != added);
        assert_eq!(first.entries, added.entries);
        fs::write(root.join("nested/_ink.scss"), "$ink: #123456").unwrap();
        let edited = scan(&root).unwrap();
        assert!(edited != added);
        fs::remove_file(root.join("nested/_ink.scss")).unwrap();
        assert!(scan(&root).unwrap() != edited);
    }
    #[test]
    fn source_ignore_rules_preserve_hidden_project_sources_and_deep_entries() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        for name in [
            "node_modules",
            "dist",
            ".vela-recovery",
            ".openshell-recovery",
            "vendor",
        ] {
            fs::create_dir(root.join(name)).unwrap();
            fs::write(root.join(name).join("skip.scss"), ".skip {}").unwrap();
        }
        let deep = root
            .join(".styles")
            .join((0..20).map(|_| "deep").collect::<PathBuf>());
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("main.sass"), ".main\n  color: red").unwrap();
        assert_eq!(scan(&root).unwrap().entries, [deep.join("main.sass")]);
    }
    #[test]
    fn configuration_edits_trigger_a_pass_without_becoming_entrypoints() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir(root.join(".vscode")).unwrap();
        fs::write(root.join(".vscode/settings.json"), "{}").unwrap();
        let first = scan(&root).unwrap();
        assert!(first.entries.is_empty());
        fs::write(root.join(".vscode/settings.json"), "{\"formats\":[]}").unwrap();
        assert!(first != scan(&root).unwrap());
        fs::write(root.join("vela-sass.json"), "{}").unwrap();
        let configured = scan(&root).unwrap();
        fs::write(root.join("unrelated.json"), "{}").unwrap();
        assert!(configured == scan(&root).unwrap());
    }
    #[cfg(unix)]
    #[test]
    fn symlink_sources_and_roots_are_not_followed() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("outside.scss"), ".outside {}").unwrap();
        std::os::unix::fs::symlink(outside.path().join("outside.scss"), root.join("link.scss"))
            .unwrap();
        std::os::unix::fs::symlink(outside.path(), root.join("linked-dir")).unwrap();
        assert!(scan(&root).unwrap().entries.is_empty());
        assert!(scan(&root.join("linked-dir")).is_err());
    }
}
