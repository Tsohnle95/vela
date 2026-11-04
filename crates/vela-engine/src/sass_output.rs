//! Install complete CSS output beside the source without following symlinks.
use std::{
    fs,
    io::{self, Write},
    path::{Component, Path},
};

fn validate(root: &Path, output: &Path, source_map: bool) -> Result<(), String> {
    let metadata = fs::symlink_metadata(root).map_err(|error| error.to_string())?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || root.canonicalize().map_err(|error| error.to_string())? != root
    {
        return Err("Sass workspace root changed; reopen it before compiling".into());
    }
    let css = output
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("css"));
    let map = output
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".css.map"));
    if if source_map { !map } else { !css } {
        return Err("Sass output must end in .css".into());
    }
    let relative = output
        .strip_prefix(root)
        .map_err(|_| "Sass output escapes the workspace")?;
    let mut current = root.to_owned();
    let components = relative.components().collect::<Vec<_>>();
    if components.is_empty() {
        return Err("Sass output must be a file".into());
    }
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err("Sass output escapes the workspace".into());
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata)
                if !metadata.file_type().is_symlink()
                    && if index + 1 == components.len() {
                        metadata.is_file()
                    } else {
                        metadata.is_dir()
                    } => {}
            Err(error)
                if error.kind() == io::ErrorKind::NotFound && index + 1 == components.len() => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match fs::create_dir(&current) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error.to_string()),
                }
                let metadata = fs::symlink_metadata(&current).map_err(|error| error.to_string())?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err("Sass output directory is not a real workspace directory".into());
                }
            }
            Ok(_) => return Err("Sass output is not a regular workspace file".into()),
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}
pub(crate) fn write_css(
    root: &Path,
    output: &Path,
    css: &str,
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<(), String> {
    write_as(root, output, css, false, || !cancel.is_cancelled())
}
pub(crate) fn write_map(
    root: &Path,
    output: &Path,
    map: &str,
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<(), String> {
    write_as(root, output, map, true, || !cancel.is_cancelled())
}
fn write_as(
    root: &Path,
    output: &Path,
    content: &str,
    source_map: bool,
    allowed: impl Fn() -> bool,
) -> Result<(), String> {
    if !allowed() {
        return Err("Sass compilation cancelled".into());
    }
    validate(root, output, source_map)?;
    if !allowed() {
        return Err("Sass compilation cancelled".into());
    }
    let directory = output.parent().ok_or("Sass output has no parent")?;
    let temporary = directory.join(format!(".vela-sass-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<(), String> {
        {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|error| error.to_string())?;
            file.write_all(content.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(|error| error.to_string())?;
        }
        validate(root, output, source_map)?;
        if !allowed() {
            return Err("Sass compilation cancelled".into());
        }
        fs::rename(&temporary, output).map_err(|error| error.to_string())
    })();
    let _ = fs::remove_file(temporary);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn write(root: &Path, output: &Path, css: &str) -> Result<(), String> {
        super::write_css(
            root,
            output,
            css,
            &tokio_util::sync::CancellationToken::new(),
        )
    }
    fn write_map(root: &Path, output: &Path, map: &str) -> Result<(), String> {
        super::write_map(
            root,
            output,
            map,
            &tokio_util::sync::CancellationToken::new(),
        )
    }
    #[test]
    fn cancelling_a_staged_output_preserves_previous_css_and_cleans_the_temp_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let output = root.join("main.css");
        fs::write(&output, "last good output").unwrap();
        let allowed = || {
            !fs::read_dir(&root).unwrap().any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".vela-sass-")
            })
        };
        assert!(write_as(&root, &output, "cancelled output", false, allowed).is_err());
        assert_eq!(fs::read_to_string(&output).unwrap(), "last good output");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    }
    #[test]
    fn mapped_output_directories_and_maps_are_installed_inside_the_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let map = root.join("public/nested/main.css.map");
        write_map(&root, &map, "{\"version\":3}").unwrap();
        write(&root, &root.join("public/nested/main.css"), ".a{}\n").unwrap();
        assert_eq!(fs::read_to_string(map).unwrap(), "{\"version\":3}");
        assert!(write_map(&root, &root.join("public/source.scss.map"), "bad").is_err());
        assert!(!root.join("public/source.scss.map").exists());
        assert_eq!(fs::read_dir(root.join("public/nested")).unwrap().count(), 2);
    }
    #[test]
    fn complete_output_replaces_old_css_without_leaving_staging_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let output = root.join("界.css");
        fs::write(&output, "old bytes").unwrap();
        write(&root, &output, ".new { color: red; }\n").unwrap();
        assert_eq!(
            fs::read_to_string(&output).unwrap(),
            ".new { color: red; }\n"
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        assert!(write(&root, &root.join("../escape.css"), "bad").is_err());
        assert!(write(&root, &root.join("source.scss"), "bad").is_err());
        fs::create_dir(root.join("directory.css")).unwrap();
        assert!(write(&root, &root.join("directory.css"), "bad").is_err());
        assert!(root.join("directory.css").is_dir());
    }
    #[cfg(unix)]
    #[test]
    fn symlink_output_parent_or_changed_root_cannot_modify_external_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let protected = outside.path().join("protected.css");
        fs::write(&protected, "protected").unwrap();
        std::os::unix::fs::symlink(&protected, root.join("link.css")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.join("linked-dir")).unwrap();
        assert!(write(&root, &root.join("link.css"), "bad").is_err());
        assert!(write(&root, &root.join("linked-dir/protected.css"), "bad").is_err());
        assert!(
            write(
                &root.join("linked-dir"),
                &root.join("linked-dir/protected.css"),
                "bad"
            )
            .is_err()
        );
        assert_eq!(fs::read_to_string(protected).unwrap(), "protected");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    }
}
