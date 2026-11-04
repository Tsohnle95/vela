//! Recoverable compare-and-write transactions, matching Vela's source-hold /
//! no-clobber installation protocol and version-1 recovery manifests.
use crate::files::{self, FileError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    version: u8,
    id: String,
    operation: String,
    original_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_path: Option<String>,
    created_at: u64,
    phase: String,
    acknowledged: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryRecord {
    pub id: String,
    pub artifact: String,
    pub original_path: String,
    pub recovery_path: PathBuf,
    pub created_at: u64,
    pub acknowledged: bool,
    pub reason: String,
}
fn invalid(message: impl Into<String>) -> FileError {
    FileError::Invalid(message.into())
}
fn clock_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn coordinator(root: &Path) -> Result<Arc<Mutex<()>>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Weak<Mutex<()>>>>> = OnceLock::new();
    let root = root.canonicalize()?;
    let mut locks = LOCKS
        .get_or_init(Mutex::default)
        .lock()
        .map_err(|_| invalid("workspace coordinator unavailable"))?;
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(&root).and_then(Weak::upgrade) {
        return Ok(lock);
    }
    let lock = Arc::new(Mutex::new(()));
    locks.insert(root, Arc::downgrade(&lock));
    Ok(lock)
}
#[cfg(unix)]
fn same_device(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev()
}
#[cfg(not(unix))]
fn same_device(_: &fs::Metadata, _: &fs::Metadata) -> bool {
    false
}
fn recovery_root(root: &Path) -> Result<PathBuf> {
    let root = root.canonicalize()?;
    let metadata = fs::metadata(&root)?;
    let git = root.join(".git");
    if let Ok(git_meta) = fs::symlink_metadata(&git)
        && git_meta.is_dir()
        && same_device(&metadata, &git_meta)
    {
        return Ok(git.join("vela-recovery"));
    }
    if let Some(parent) = root.parent()
        && same_device(&metadata, &fs::metadata(parent)?)
    {
        let key = format!("{:x}", Sha256::digest(root.to_string_lossy().as_bytes()));
        return Ok(parent.join(format!(".vela-recovery-{}", &key[..16])));
    }
    Ok(root.join(".vela-recovery"))
}
fn private_dir(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(invalid("invalid recovery directory")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(path)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}
fn sync_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    Ok(())
}
fn create_synced(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn write_manifest(directory: &Path, transaction: &Transaction) -> Result<()> {
    let path = directory.join(format!("manifest-{}.tmp", Uuid::new_v4()));
    create_synced(
        &path,
        &serde_json::to_vec_pretty(transaction).map_err(|error| invalid(error.to_string()))?,
    )?;
    fs::rename(path, directory.join("manifest.json"))?;
    sync_dir(directory)
}
fn create_transaction(root: &Path, rel: &str) -> Result<(PathBuf, Transaction)> {
    create_transaction_for(root, "save", rel, None)
}
fn create_transaction_for(
    root: &Path,
    operation: &str,
    rel: &str,
    target: Option<String>,
) -> Result<(PathBuf, Transaction)> {
    let recovery = recovery_root(root)?;
    private_dir(&recovery)?;
    let created_at = clock_ms();
    let id = format!("{created_at}-{}", Uuid::new_v4());
    let directory = recovery.join(&id);
    private_dir(&directory)?;
    let transaction = Transaction {
        version: 1,
        id,
        operation: operation.into(),
        original_path: rel.into(),
        target_path: target,
        created_at,
        phase: "initialized".into(),
        acknowledged: vec![],
        reason: None,
    };
    write_manifest(&directory, &transaction)?;
    sync_dir(&recovery)?;
    Ok((directory, transaction))
}
/// Resolve a possibly missing target without following a newly introduced
/// symlink. Source-hold recovery must never write outside its original root.
fn target_path(root: &Path, rel: &str) -> Result<PathBuf> {
    files::normalize_rel(rel)?;
    if rel.is_empty() {
        return Err(invalid("file path is empty"));
    }
    let root = root.canonicalize()?;
    let target = root.join(rel);
    let parent = target
        .parent()
        .ok_or_else(|| invalid("file has no parent"))?
        .canonicalize()?;
    if !parent.starts_with(&root) {
        return Err(invalid("path escapes workspace"));
    }
    if let Ok(meta) = fs::symlink_metadata(&target)
        && !meta.is_file()
    {
        return Err(invalid("target is not a regular file"));
    }
    Ok(target)
}
fn transactions(root: &Path) -> Result<Vec<(PathBuf, Transaction)>> {
    let current = recovery_root(root)?;
    let mut locations = vec![current, root.join(".vela-recovery")];
    locations.sort();
    locations.dedup();
    let mut transactions = vec![];
    for location in locations {
        let Ok(meta) = fs::symlink_metadata(&location) else {
            continue;
        };
        if !meta.is_dir() {
            continue;
        }
        for entry in fs::read_dir(location)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            let Some((stamp, uuid)) = id.split_once('-') else {
                continue;
            };
            if stamp.len() != 13 || stamp.parse::<u64>().is_err() || Uuid::parse_str(uuid).is_err()
            {
                continue;
            }
            let manifest = entry.path().join("manifest.json");
            let Ok(meta) = fs::symlink_metadata(&manifest) else {
                continue;
            };
            if !meta.is_file() || meta.len() > 65536 {
                continue;
            }
            let Ok(transaction) = serde_json::from_slice::<Transaction>(&fs::read(manifest)?)
            else {
                continue;
            };
            if transaction.version != 1
                || transaction.id != id
                || !["save", "rename"].contains(&transaction.operation.as_str())
                || (transaction.operation == "rename"
                    && !transaction
                        .target_path
                        .as_ref()
                        .is_some_and(|target| !target.is_empty() && files::is_safe_rel(target)))
                || transaction.created_at == 0
                || !files::is_safe_rel(&transaction.original_path)
                || transaction.original_path.is_empty()
                || ![
                    "initialized",
                    "temporary-ready",
                    "source-held",
                    "held-validated",
                    "target-installed",
                    "complete",
                    "failed",
                ]
                .contains(&transaction.phase.as_str())
                || transaction.acknowledged.iter().any(|s| {
                    !["temporary", "original", "proposed", "rename-source"].contains(&s.as_str())
                })
            {
                continue;
            }
            transactions.push((entry.path(), transaction));
        }
    }
    Ok(transactions)
}
fn reconcile_locked(root: &Path) -> Result<()> {
    for (directory, mut transaction) in transactions(root)? {
        if transaction.operation == "rename" {
            reconcile_rename(root, &directory, &mut transaction)?;
            continue;
        }
        if !["source-held", "held-validated"].contains(&transaction.phase.as_str()) {
            continue;
        }
        let original = directory.join("original");
        if !fs::symlink_metadata(&original).is_ok_and(|m| m.is_file()) {
            continue;
        }
        let target = target_path(root, &transaction.original_path)?;
        if target.try_exists()? {
            continue;
        }
        match fs::hard_link(original, &target) {
            Ok(()) => {
                sync_dir(target.parent().unwrap())?;
                transaction.phase = "complete".into();
                transaction.reason = Some("crash-recovered".into());
                write_manifest(&directory, &transaction)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
pub fn reconcile(root: &Path) -> Result<()> {
    let lock = coordinator(root)?;
    let _guard = lock
        .lock()
        .map_err(|_| invalid("workspace coordinator unavailable"))?;
    reconcile_locked(root)
}
fn initialize_locked(root: &Path) -> Result<()> {
    static INITIALIZED: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    let root = root.canonicalize()?;
    let initialized = INITIALIZED.get_or_init(Mutex::default);
    if initialized
        .lock()
        .map_err(|_| invalid("recovery registry unavailable"))?
        .contains(&root)
    {
        return Ok(());
    }
    let _ = migrate_legacy_locked(&root);
    reconcile_locked(&root)?;
    let _ = purge_locked(&root);
    initialized
        .lock()
        .map_err(|_| invalid("recovery registry unavailable"))?
        .insert(root);
    Ok(())
}
/// Reconcile once per workspace activation in this process. Normal file reads
/// and saves must not rescan an ever-growing recovery journal on every action.
pub fn initialize(root: &Path) -> Result<()> {
    let lock = coordinator(root)?;
    let _guard = lock
        .lock()
        .map_err(|_| invalid("workspace coordinator unavailable"))?;
    initialize_locked(root)
}

/// Build an external copy privately, then claim its final name without replacing
/// an entry that appeared during the copy. Sources are never moved or removed.
pub(crate) fn install_import<T>(root:&Path,rel:&str,build:impl FnOnce(&Path)->Result<T>)->Result<T> {
    struct Stage(PathBuf);
    impl Drop for Stage {fn drop(&mut self) {let _=fs::remove_dir_all(&self.0);}}
    let lock=coordinator(root)?;
    let _guard=lock.lock().map_err(|_|invalid("workspace coordinator unavailable"))?;
    initialize_locked(root)?;
    let target=files::new_entry_path(root,rel)?;
    if fs::symlink_metadata(&target).is_ok() {return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists).into());}
    let mut recovery=recovery_root(root)?;
    private_dir(&recovery)?;
    #[cfg(unix)]
    if !same_device(&fs::metadata(target.parent().unwrap())?,&fs::metadata(&recovery)?) {
        // Imported sources can cross devices; the final staged install cannot.
        // Keep staging on the destination mount using the existing hidden store.
        recovery=target.parent().unwrap().join(".vela-recovery");
        private_dir(&recovery)?;
    }
    let path=recovery.join(format!("import-{}",Uuid::new_v4()));
    let mut builder=fs::DirBuilder::new();
    #[cfg(unix)] {use std::os::unix::fs::DirBuilderExt;builder.mode(0o700);}
    builder.create(&path)?;
    let stage=Stage(path);
    let entry=stage.0.join("entry");
    let result=build(&entry)?;
    // Re-resolve the parent after copying rather than trusting the initial path.
    let current=files::new_entry_path(root,rel)?;
    if current!=target {return Err(invalid("import destination changed"));}
    let metadata=fs::symlink_metadata(&entry)?;
    if metadata.is_dir() {rename_directory_exclusive(&entry,&target)?;}
    else if metadata.is_file() {fs::hard_link(&entry,&target)?;}
    else {return Err(invalid("unsupported import entry"));}
    sync_dir(target.parent().unwrap())?;
    Ok(result)
}

pub(crate) fn detach_entry(root:&Path,rel:&str,target:&Path)->Result<()> {
    let lock=coordinator(root)?;
    let _guard=lock.lock().map_err(|_|invalid("workspace coordinator unavailable"))?;
    initialize_locked(root)?;
    let source=files::confine(root,rel)?;
    rename_directory_exclusive(&source,target)?;
    sync_dir(source.parent().unwrap())?;
    sync_dir(target.parent().unwrap())?;
    Ok(())
}
pub fn records(root: &Path) -> Result<Vec<RecoveryRecord>> {
    let mut records = vec![];
    for (directory, transaction) in transactions(root)? {
        for artifact in if transaction.operation == "rename" {
            vec!["rename-source"]
        } else {
            vec!["temporary", "original", "proposed"]
        } {
            let path = directory.join(if artifact == "rename-source" {
                "source"
            } else {
                artifact
            });
            if !fs::symlink_metadata(&path).is_ok_and(|m| m.is_file()) {
                continue;
            }
            records.push(RecoveryRecord {
                id: format!("{}:{artifact}", transaction.id),
                artifact: artifact.into(),
                original_path: transaction.original_path.clone(),
                recovery_path: path,
                created_at: transaction.created_at,
                acknowledged: transaction.acknowledged.iter().any(|s| s == artifact),
                reason: transaction
                    .reason
                    .clone()
                    .unwrap_or_else(|| "save-failed".into()),
            });
        }
    }
    records.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(records)
}

/// Mark one preserved artifact as reviewed without deleting its bytes.
pub fn acknowledge(root: &Path, id: &str) -> Result<()> {    let (transaction_id, artifact) = id
        .rsplit_once(':')
        .ok_or_else(|| invalid("invalid recovery record"))?;
    if !["temporary", "original", "proposed", "rename-source"].contains(&artifact) {
        return Err(invalid("invalid recovery record"));
    }
    let lock = coordinator(root)?;
    let _guard = lock
        .lock()
        .map_err(|_| invalid("workspace coordinator unavailable"))?;
    let Some((directory, mut transaction)) = transactions(root)?
        .into_iter()
        .find(|(_, transaction)| transaction.id == transaction_id)
    else {
        return Err(invalid("recovery record not found"));
    };
    let file = directory.join(if artifact == "rename-source" {
        "source"
    } else {
        artifact
    });
    if !fs::symlink_metadata(&file).is_ok_and(|metadata| metadata.is_file()) {
        return Err(invalid("recovery record not found"));
    }
    if !transaction.acknowledged.iter().any(|item| item == artifact) {
        transaction.acknowledged.push(artifact.into());
        write_manifest(&directory, &transaction)?;
        sync_dir(&directory)?;
    }
    Ok(())
}

/// Remove settled transactions after one day and interrupted ones after a
/// week, matching the reference retention windows.
pub fn purge(root: &Path) -> Result<()> {
    let lock = coordinator(root)?;
    let _guard = lock
        .lock()
        .map_err(|_| invalid("workspace coordinator unavailable"))?;
    purge_locked(root)
}

fn purge_locked(root: &Path) -> Result<()> {
    const SETTLED: u64 = 24 * 60 * 60 * 1000;
    const INTERRUPTED: u64 = 7 * 24 * 60 * 60 * 1000;
    let now = clock_ms();
    for (directory, transaction) in transactions(root)? {
        let settled = (transaction.phase == "complete" || transaction.phase == "failed")
            || !transaction.acknowledged.is_empty();
        let interrupted = ["source-held", "held-validated"].contains(&transaction.phase.as_str());
        if !settled && !interrupted {
            continue;
        }
        let retention = if settled { SETTLED } else { INTERRUPTED };
        if now.saturating_sub(transaction.created_at) < retention {
            continue;
        }
        fs::remove_dir_all(&directory)?;
    }
    Ok(())
}

const LEGACY_RECOVERY_NAMES: [&str; 4] = [
    ".vela-recovery",
    ".mira-recovery",
    ".openshell-recovery",
    ".orbit-recovery",
];

fn migrate_legacy_locked(root: &Path) -> Result<()> {
    let destination = recovery_root(root)?;
    private_dir(&destination)?;
    let canonical_destination = destination.canonicalize()?;
    for name in LEGACY_RECOVERY_NAMES {
        let legacy = root.join(name);
        if !fs::symlink_metadata(&legacy).is_ok_and(|metadata| metadata.is_dir()) {
            continue;
        }
        if legacy
            .canonicalize()
            .is_ok_and(|path| path == canonical_destination)
        {
            continue;
        }
        for entry in fs::read_dir(&legacy)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            let Some((stamp, uuid)) = id.split_once('-') else {
                continue;
            };
            if stamp.len() != 13 || stamp.parse::<u64>().is_err() || Uuid::parse_str(uuid).is_err() {
                continue;
            }
            let manifest = entry.path().join("manifest.json");
            if !fs::symlink_metadata(&manifest).is_ok_and(|metadata| metadata.is_file()) {
                continue;
            }
            let target = destination.join(&id);
            if fs::symlink_metadata(&target).is_ok() {
                continue;
            }
            fs::rename(entry.path(), target)?;
        }
        let _ = fs::remove_dir(&legacy);
        sync_dir(&destination)?;
    }
    Ok(())
}

/// Import transactions from older recovery directory names without overwriting
/// current transactions. Source directories are left in place if migration
/// cannot complete for an individual entry.
pub fn migrate_legacy(root: &Path) -> Result<()> {
    let lock = coordinator(root)?;
    let _guard = lock
        .lock()
        .map_err(|_| invalid("workspace coordinator unavailable"))?;
    migrate_legacy_locked(root)
}

fn same_file(a: &Path, b: &Path) -> bool {
    fs::symlink_metadata(a)
        .ok()
        .zip(fs::symlink_metadata(b).ok())
        .is_some_and(|(a, b)| {
            a.is_file()
                && b.is_file()
                && files::file_identity(&a)
                    .zip(files::file_identity(&b))
                    .is_some_and(|(a, b)| a == b)
        })
}

fn reconcile_rename(root: &Path, directory: &Path, transaction: &mut Transaction) -> Result<()> {
    if ["complete", "failed"].contains(&transaction.phase.as_str()) {
        return Ok(());
    }
    let held = directory.join("source");
    if !fs::symlink_metadata(&held).is_ok_and(|metadata| metadata.is_file()) {
        return Ok(());
    }
    let destination = target_path(
        root,
        transaction
            .target_path
            .as_deref()
            .ok_or_else(|| invalid("rename target is missing"))?,
    )?;
    if same_file(&held, &destination) {
        transaction.phase = "complete".into();
        transaction.reason = Some("renamed".into());
        transaction.acknowledged = vec!["rename-source".into()];
    } else {
        let original = target_path(root, &transaction.original_path)?;
        match fs::hard_link(&held, &original) {
            Ok(()) => sync_dir(original.parent().unwrap())?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        transaction.phase = "failed".into();
        transaction.reason = Some("crash-recovered".into());
    }
    write_manifest(directory, transaction)
}

/// A destination created after inspection must survive directory relocation.
/// Do not fall back to a replacing rename when the platform lacks this API.
fn rename_directory_exclusive(source: &Path, target: &Path) -> std::io::Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::ffi::OsStrExt;
        let source = std::ffi::CString::new(source.as_os_str().as_bytes())?;
        let target = std::ffi::CString::new(target.as_os_str().as_bytes())?;
        // SAFETY: both pointers refer to live, NUL-terminated path strings.
        let result = unsafe {
            #[cfg(target_os = "macos")]
            {
                libc::renamex_np(source.as_ptr(), target.as_ptr(), libc::RENAME_EXCL)
            }
            #[cfg(target_os = "linux")]
            {
                libc::renameat2(
                    libc::AT_FDCWD,
                    source.as_ptr(),
                    libc::AT_FDCWD,
                    target.as_ptr(),
                    libc::RENAME_NOREPLACE,
                )
            }
        };
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name="kernel32")]
        unsafe extern "system" {fn MoveFileW(source:*const u16,target:*const u16)->i32;}
        let source=source.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        let target=target.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        // SAFETY: both vectors are live, NUL-terminated UTF-16 paths. MoveFileW
        // rejects an occupied destination for both files and directories.
        if unsafe {MoveFileW(source.as_ptr(),target.as_ptr())}!=0 {Ok(())}
        else {Err(std::io::Error::last_os_error())}
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        let _ = (source, target);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "exclusive directory move is unavailable",
        ))
    }
}

pub fn rename_entry(root: &Path, rel: &str, new_name: &str) -> Result<String> {
    rename_with_hook(root, rel, new_name, |_, _, _| Ok(()))
}
fn rename_with_hook(
    root: &Path,
    rel: &str,
    new_name: &str,
    phase: impl FnMut(&str, &Path, &Path) -> Result<()>,
) -> Result<String> {
    let new_name = new_name.trim();
    if new_name.is_empty()
        || new_name.contains('/')
        || new_name.contains('\\')
        || new_name.contains('\0')
        || [".", ".."].contains(&new_name)
    {
        return Err(invalid("Invalid name"));
    }
    files::normalize_rel(rel)?;
    if rel.is_empty() {
        return Err(invalid("cannot rename the workspace root"));
    }
    let parent = rel.rsplit_once('/').map(|(parent, _)| parent).unwrap_or("");
    let new_rel = if parent.is_empty() {
        new_name.to_owned()
    } else {
        format!("{parent}/{new_name}")
    };
    relocate_with_hook(root, rel, &new_rel, phase)
}

/// Move an existing entry into an existing folder using the same source-hold
/// transaction as rename. The destination keeps the exact source basename.
pub fn move_entry(root: &Path, rel: &str, parent: &str) -> Result<String> {
    files::normalize_rel(rel)?;
    files::normalize_rel(parent)?;
    if rel.is_empty() {
        return Err(invalid("cannot move the workspace root"));
    }
    if parent == rel
        || parent
            .strip_prefix(rel)
            .is_some_and(|tail| tail.starts_with('/'))
    {
        return Err(invalid("cannot move a folder into itself"));
    }
    let protected = |path: &str| {
        path.split('/').any(|part| {
            matches!(
                part,
                ".vela-recovery" | ".mira-recovery" | ".openshell-recovery" | ".orbit-recovery"
            ) || part == "vela-recovery" && path.starts_with(".git/")
        })
    };
    if protected(rel) || protected(parent) {
        return Err(invalid("cannot move recovery storage"));
    }
    let name = rel.rsplit('/').next().unwrap();
    let target = if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    };
    if target == rel {
        return Err(invalid("entry is already in that folder"));
    }
    relocate_with_hook(root, rel, &target, |_, _, _| Ok(()))
}

fn relocate_with_hook(
    root: &Path,
    rel: &str,
    new_rel: &str,
    mut phase: impl FnMut(&str, &Path, &Path) -> Result<()>,
) -> Result<String> {
    let new_rel = new_rel.to_owned();
    let lock = coordinator(root)?;
    let _guard = lock
        .lock()
        .map_err(|_| invalid("workspace coordinator unavailable"))?;
    initialize_locked(root)?;
    let original = files::confine(root, rel)?;
    let target = target_path(root, &new_rel)?;
    if original == target {
        return Ok(new_rel);
    }
    let inspected = fs::symlink_metadata(&original)?;
    #[cfg(unix)]
    if !same_device(&inspected, &fs::metadata(target.parent().unwrap())?) {
        return Err(invalid("cannot move across filesystems"));
    }
    let same_existing_file = inspected.is_file()
        && (same_file(&original, &target)
            || fs::canonicalize(&original)
                .ok()
                .zip(fs::canonicalize(&target).ok())
                .is_some_and(|(a, b)| a == b));
    if fs::symlink_metadata(&target).is_ok() && !same_existing_file {
        return Err(invalid("destination already exists"));
    }
    if inspected.is_dir() {
        phase("directory-inspected", &original, &target)?;
        rename_directory_exclusive(&original, &target)?;
        sync_dir(original.parent().unwrap())?;
        sync_dir(target.parent().unwrap())?;
        return Ok(new_rel);
    }
    if !inspected.is_file() {
        return Err(invalid("source is not a regular file"));
    }
    phase("source-inspected", &original, &target)?;
    let (directory, mut transaction) =
        create_transaction_for(root, "rename", rel, Some(new_rel.clone()))?;
    let held = directory.join("source");
    let (mut source_held, mut installed) = (false, false);
    let result = (|| {
        fs::rename(&original, &held)?;
        source_held = true;
        sync_dir(original.parent().unwrap())?;
        sync_dir(&directory)?;
        transaction.phase = "source-held".into();
        write_manifest(&directory, &transaction)?;
        phase("source-held", &held, &original)?;
        if !fs::symlink_metadata(&held)?.is_file() {
            return Err(invalid("source changed during rename"));
        }
        let target = target_path(root, &new_rel)?;
        fs::hard_link(&held, &target)?;
        installed = true;
        sync_dir(target.parent().unwrap())?;
        transaction.phase = "target-installed".into();
        write_manifest(&directory, &transaction)?;
        phase("target-installed", &held, &target)?;
        transaction.phase = "complete".into();
        transaction.reason = Some("renamed".into());
        transaction.acknowledged = vec!["rename-source".into()];
        write_manifest(&directory, &transaction)
    })();
    if let Err(error) = result {
        if source_held && !installed {
            let _ = fs::hard_link(&held, &original);
            let _ = sync_dir(original.parent().unwrap());
        }
        transaction.phase = "failed".into();
        transaction.reason = Some("rename-failed".into());
        let _ = write_manifest(&directory, &transaction);
        return Err(invalid(format!(
            "{error}; recovery artifacts preserved in {}",
            directory.display()
        )));
    }
    Ok(new_rel)
}

pub fn write_file(
    root: &Path,
    rel: &str,
    content: &str,
    expected: &str,
    overwrite: bool,
) -> Result<()> {
    write_file_with_source(root, rel, content, expected, overwrite).map(|_| ())
}

#[derive(Debug)]
pub(crate) struct WriteSource {
    pub content: Option<String>,
    pub existed: bool,
}

pub(crate) fn write_file_with_source(
    root: &Path,
    rel: &str,
    content: &str,
    expected: &str,
    overwrite: bool,
) -> Result<WriteSource> {
    write_with_hook(root, rel, content, expected, overwrite, |_, _, _| Ok(()))
}
fn write_with_hook(
    root: &Path,
    rel: &str,
    content: &str,
    expected: &str,
    overwrite: bool,
    mut phase: impl FnMut(&str, &Path, &Path) -> Result<()>,
) -> Result<WriteSource> {
    if content.len() > files::MAX_WRITE_BYTES || expected.len() > files::MAX_WRITE_BYTES {
        return Err(invalid("file is too large to save"));
    }
    let lock = coordinator(root)?;
    let _guard = lock
        .lock()
        .map_err(|_| invalid("workspace coordinator unavailable"))?;
    initialize_locked(root)?;
    let target = target_path(root, rel)?;
    let metadata = match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.is_file() => Some(metadata),
        Ok(_) => return Err(invalid("target is not a regular file")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let (directory, mut transaction) = create_transaction(root, rel)?;
    let temporary = directory.join("temporary");
    let held = directory.join("original");
    let proposed = directory.join("proposed");
    let mut source_held = false;
    let mut installed = false;
    let mut source = WriteSource {
        content: Some(String::new()),
        existed: false,
    };
    let result = (|| {
        create_synced(&temporary, content.as_bytes())?;
        create_synced(&proposed, content.as_bytes())?;
        if let Some(metadata) = &metadata {
            fs::set_permissions(&temporary, metadata.permissions())?;
        }
        transaction.phase = "temporary-ready".into();
        write_manifest(&directory, &transaction)?;
        phase("temporary-ready", &target, &temporary)?;
        target_path(root, rel)?;
        match fs::rename(&target, &held) {
            Ok(()) => {
                source_held = true;
                sync_dir(target.parent().unwrap())?;
                sync_dir(&directory)?;
                transaction.phase = "source-held".into();
                write_manifest(&directory, &transaction)?;
                phase("source-held", &held, &target)?;
                if !fs::symlink_metadata(&held)?.is_file() {
                    return Err(FileError::Changed("file changed on disk".into()));
                }
                let mut before = Vec::new();
                File::open(&held)?
                    .take(files::MAX_READ_BYTES + 1)
                    .read_to_end(&mut before)?;
                if !overwrite && before != expected.as_bytes() {
                    return Err(FileError::Changed("file changed on disk".into()));
                }
                source.existed = true;
                source.content = (before.len() as u64 <= files::MAX_READ_BYTES)
                    .then(|| String::from_utf8(before).ok())
                    .flatten();
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if !overwrite {
                    return Err(FileError::Changed("file was deleted on disk".into()));
                }
            }
            Err(error) => return Err(error.into()),
        }
        transaction.phase = "held-validated".into();
        write_manifest(&directory, &transaction)?;
        phase("held-validated", &held, &target)?;
        // hard_link is exclusive: an external creation at target is preserved.
        fs::hard_link(&temporary, &target).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                FileError::Changed("file was created during save".into())
            } else {
                error.into()
            }
        })?;
        installed = true;
        sync_dir(target.parent().unwrap())?;
        transaction.phase = "target-installed".into();
        write_manifest(&directory, &transaction)?;
        phase("target-installed", &temporary, &target)?;
        if !fs::symlink_metadata(&target)?.is_file() || fs::read(&target)? != content.as_bytes() {
            return Err(FileError::Changed("file changed during save".into()));
        }
        transaction.phase = "complete".into();
        transaction.reason = Some("saved".into());
        transaction.acknowledged = vec!["temporary".into(), "original".into(), "proposed".into()];
        write_manifest(&directory, &transaction)
    })();
    if let Err(error) = result {
        if source_held && !installed {
            let _ = fs::hard_link(&held, &target);
            let _ = sync_dir(target.parent().unwrap());
        }
        transaction.phase = "failed".into();
        transaction.reason = Some("save-failed".into());
        let _ = write_manifest(&directory, &transaction);
        let detail = format!(
            "{error}; recovery artifacts preserved in {}",
            directory.display()
        );
        return Err(if matches!(error, FileError::Changed(_)) {
            FileError::Changed(detail)
        } else {
            invalid(detail)
        });
    }
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observation_receipt_identifies_the_held_original_and_deleted_recreation() {
        let temp = fixture();
        let source = write_with_hook(
            temp.path(),
            "file.txt",
            "saved",
            "expected",
            true,
            |phase, held, _| {
                if phase == "source-held" {
                    fs::write(held, "external held source")?;
                }
                Ok(())
            },
        )
        .unwrap();
        assert!(source.existed);
        assert_eq!(source.content.as_deref(), Some("external held source"));
        fs::remove_file(temp.path().join("file.txt")).unwrap();
        let source =
            write_file_with_source(temp.path(), "file.txt", "recreated", "saved", true).unwrap();
        assert!(!source.existed);
        assert_eq!(source.content.as_deref(), Some(""));
    }
    fn fixture() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();
        fs::write(temp.path().join("file.txt"), "expected").unwrap();
        temp
    }
    #[test]
    fn first_creation_reconciles_interrupted_rename_before_claiming_the_original_name() {
        for directory in [false, true] {
            let temp = fixture();
            let root = temp.path();
            let (journal, mut transaction) =
                create_transaction_for(root, "rename", "file.txt", Some("later.txt".into()))
                    .unwrap();
            fs::rename(root.join("file.txt"), journal.join("source")).unwrap();
            transaction.phase = "source-held".into();
            write_manifest(&journal, &transaction).unwrap();
            let result = if directory {
                files::create_directory(root, "file.txt")
            } else {
                files::create_file(root, "file.txt")
            };
            assert!(
                result.is_err(),
                "creation must not claim the interrupted source name"
            );
            assert_eq!(
                fs::read_to_string(root.join("file.txt")).unwrap(),
                "expected"
            );
            assert!(!root.join("later.txt").exists());
        }
    }

    #[test]
    fn concurrent_directory_destination_creation_is_never_replaced() {
        let temp = fixture();
        let root = temp.path();
        fs::create_dir(root.join("source")).unwrap();
        fs::write(root.join("source/child"), "retained").unwrap();
        let result = rename_with_hook(root, "source", "target", |phase, _, target| {
            if phase == "directory-inspected" {
                fs::create_dir(target)?;
            }
            Ok(())
        });
        assert!(result.is_err());
        assert!(root.join("target").is_dir());
        assert_eq!(
            fs::read_to_string(root.join("source/child")).unwrap(),
            "retained"
        );
        assert!(!root.join("target/child").exists());
    }

    #[test]
    fn moves_preserve_files_and_folder_children_and_reject_unsafe_destinations() {
        let temp = fixture();
        let root = temp.path();
        fs::create_dir(root.join("destination")).unwrap();
        assert_eq!(
            move_entry(root, "file.txt", "destination").unwrap(),
            "destination/file.txt"
        );
        assert!(!root.join("file.txt").exists());
        assert_eq!(
            fs::read_to_string(root.join("destination/file.txt")).unwrap(),
            "expected"
        );
        assert_eq!(
            move_entry(root, "destination/file.txt", "").unwrap(),
            "file.txt"
        );
        fs::create_dir_all(root.join("tree/child")).unwrap();
        fs::write(root.join("tree/child/界🌍.txt"), "child bytes").unwrap();
        for parent in [
            "tree",
            "tree/child",
            "../outside",
            "missing",
            "file.txt",
            ".git/vela-recovery",
        ] {
            assert!(move_entry(root, "tree", parent).is_err(), "{parent}");
            assert_eq!(
                fs::read_to_string(root.join("tree/child/界🌍.txt")).unwrap(),
                "child bytes"
            );
        }
        assert!(move_entry(root, "file.txt", "").is_err());
        assert!(move_entry(root, "", "destination").is_err());
        fs::write(root.join("destination/file.txt"), "occupied").unwrap();
        assert!(move_entry(root, "file.txt", "destination").is_err());
        assert_eq!(
            fs::read_to_string(root.join("file.txt")).unwrap(),
            "expected"
        );
        assert_eq!(
            fs::read_to_string(root.join("destination/file.txt")).unwrap(),
            "occupied"
        );
        assert_eq!(
            move_entry(root, "tree", "destination").unwrap(),
            "destination/tree"
        );
        assert!(!root.join("tree").exists());
        assert_eq!(
            fs::read_to_string(root.join("destination/tree/child/界🌍.txt")).unwrap(),
            "child bytes"
        );
        assert_eq!(move_entry(root, "destination/tree", "").unwrap(), "tree");
    }

    #[cfg(unix)]
    #[test]
    fn moves_reject_symlink_escape_without_mutating_source_or_outside_files() {
        let temp = fixture();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("escape")).unwrap();
        assert!(move_entry(temp.path(), "file.txt", "escape").is_err());
        assert!(temp.path().join("file.txt").is_file());
        assert!(!outside.path().join("file.txt").exists());
    }

    #[test]
    fn rename_preserves_source_and_records_acknowledged_unicode_destination() {
        let temp = fixture();
        assert_eq!(
            rename_entry(temp.path(), "file.txt", "界🌍.txt").unwrap(),
            "界🌍.txt"
        );
        assert!(!temp.path().join("file.txt").exists());
        assert_eq!(
            fs::read_to_string(temp.path().join("界🌍.txt")).unwrap(),
            "expected"
        );
        let records = records(temp.path()).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].artifact, "rename-source");
        assert!(records[0].acknowledged);
        assert_eq!(records[0].reason, "renamed");
        assert_eq!(
            fs::read_to_string(&records[0].recovery_path).unwrap(),
            "expected"
        );
        fs::create_dir(temp.path().join("before")).unwrap();
        fs::write(temp.path().join("before/child"), "child").unwrap();
        assert_eq!(
            rename_entry(temp.path(), "before", "after").unwrap(),
            "after"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("after/child")).unwrap(),
            "child"
        );
    }

    #[test]
    fn file_case_only_rename_keeps_source_on_case_insensitive_volumes() {
        let temp = fixture();
        rename_entry(temp.path(), "file.txt", "FILE.TXT").unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("FILE.TXT")).unwrap(),
            "expected"
        );
        assert_eq!(
            fs::read_dir(temp.path())
                .unwrap()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_name() == "FILE.TXT")
                .count(),
            1
        );
    }

    #[test]
    fn rename_rejects_collisions_and_invalid_targets_without_losing_source() {
        let temp = fixture();
        fs::write(temp.path().join("occupied"), "other").unwrap();
        assert!(rename_entry(temp.path(), "file.txt", "occupied").is_err());
        for name in ["", "..", "../escape", "a/b", "a\\b", "bad\0name"] {
            assert!(rename_entry(temp.path(), "file.txt", name).is_err());
        }
        assert!(rename_entry(temp.path(), "", "renamed-root").is_err());
        assert_eq!(
            fs::read_to_string(temp.path().join("file.txt")).unwrap(),
            "expected"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("occupied")).unwrap(),
            "other"
        );
        fs::create_dir(temp.path().join("dir-a")).unwrap();
        fs::create_dir(temp.path().join("dir-b")).unwrap();
        assert!(rename_entry(temp.path(), "dir-a", "dir-b").is_err());
        assert!(temp.path().join("dir-a").is_dir());
        assert!(temp.path().join("dir-b").is_dir());
    }

    #[test]
    fn rename_external_source_and_destination_creations_never_get_overwritten() {
        let temp = fixture();
        let result = rename_with_hook(
            temp.path(),
            "file.txt",
            "target.txt",
            |phase, _, original| {
                if phase == "source-held" {
                    fs::write(original, "new source")?;
                    fs::write(temp.path().join("target.txt"), "new destination")?;
                }
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(temp.path().join("file.txt")).unwrap(),
            "new source"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("target.txt")).unwrap(),
            "new destination"
        );
        let records = records(temp.path()).unwrap();
        assert_eq!(records.len(), 1);
        assert!(!records[0].acknowledged);
        assert_eq!(
            fs::read_to_string(&records[0].recovery_path).unwrap(),
            "expected"
        );
    }

    #[test]
    fn interrupted_rename_restores_held_source_and_exposes_recovery() {
        let temp = fixture();
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            rename_with_hook(temp.path(), "file.txt", "target.txt", |phase, _, _| {
                if phase == "source-held" {
                    panic!("simulated process loss");
                }
                Ok(())
            })
        }));
        assert!(stopped.is_err());
        assert!(!temp.path().join("file.txt").exists());
        reconcile(temp.path()).unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("file.txt")).unwrap(),
            "expected"
        );
        assert!(!temp.path().join("target.txt").exists());
        let records = records(temp.path()).unwrap();
        assert!(!records[0].acknowledged);
        assert_eq!(records[0].reason, "crash-recovered");
    }

    #[test]
    fn recovery_acknowledgement_preserves_artifact_bytes_and_rejects_unknown_ids() {
        let temp = fixture();
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            rename_with_hook(temp.path(), "file.txt", "target.txt", |phase, _, _| {
                if phase == "source-held" {
                    panic!("simulated process loss");
                }
                Ok(())
            })
        }));
        assert!(stopped.is_err());
        reconcile(temp.path()).unwrap();
        let record = records(temp.path()).unwrap().remove(0);
        assert!(!record.acknowledged);
        acknowledge(temp.path(), &record.id).unwrap();
        let acknowledged = records(temp.path()).unwrap().remove(0);
        assert!(acknowledged.acknowledged);
        assert_eq!(
            fs::read_to_string(acknowledged.recovery_path).unwrap(),
            "expected"
        );
        assert!(acknowledge(temp.path(), "missing:original").is_err());
        assert!(acknowledge(temp.path(), "../../outside:original").is_err());
        assert!(acknowledge(temp.path(), &format!("{}:unknown", record.id.split_once(':').unwrap().0)).is_err());
    }

    #[test]
    fn legacy_recovery_directories_are_imported_without_overwriting_current_transactions() {
        let temp = fixture();
        let (directory, _) = create_transaction(temp.path(), "file.txt").unwrap();
        let id = directory.file_name().unwrap().to_string_lossy().to_string();
        fs::write(directory.join("proposed"), "legacy bytes").unwrap();
        let legacy = temp.path().join(".mira-recovery");
        fs::create_dir(&legacy).unwrap();
        fs::rename(&directory, legacy.join(&id)).unwrap();
        migrate_legacy(temp.path()).unwrap();
        assert!(records(temp.path()).unwrap().iter().any(|record| record.id.starts_with(&id)));
        assert!(!legacy.join(&id).exists());
        assert!(!legacy.exists());
    }

    #[test]
    fn retention_purges_old_settled_and_interrupted_transactions_only() {        let temp = fixture();
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            rename_with_hook(temp.path(), "file.txt", "target.txt", |phase, _, _| {
                if phase == "source-held" {
                    panic!("simulated process loss");
                }
                Ok(())
            })
        }));
        assert!(stopped.is_err());
        reconcile(temp.path()).unwrap();
        let mut entries = transactions(temp.path()).unwrap();
        assert_eq!(entries.len(), 1);
        {
            let (directory, transaction) = &mut entries[0];
            transaction.created_at = clock_ms().saturating_sub(8 * 24 * 60 * 60 * 1000);
            write_manifest(directory, transaction).unwrap();
        }
        purge(temp.path()).unwrap();
        assert!(records(temp.path()).unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn interrupted_installed_rename_is_completed_without_recreating_old_name() {
        let temp = fixture();
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            rename_with_hook(temp.path(), "file.txt", "target.txt", |phase, _, _| {
                if phase == "target-installed" {
                    panic!("simulated process loss");
                }
                Ok(())
            })
        }));
        assert!(stopped.is_err());
        reconcile(temp.path()).unwrap();
        assert!(!temp.path().join("file.txt").exists());
        assert_eq!(
            fs::read_to_string(temp.path().join("target.txt")).unwrap(),
            "expected"
        );
        assert!(records(temp.path()).unwrap()[0].acknowledged);
    }

    #[test]
    fn explicit_overwrite_preserves_the_external_original_and_can_recreate_a_deleted_file() {
        let temp = fixture();
        let target = temp.path().join("file.txt");
        fs::write(&target, "external").unwrap();
        write_file(temp.path(), "file.txt", "merged", "expected", true).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "merged");
        assert!(
            records(temp.path())
                .unwrap()
                .iter()
                .any(|r| r.artifact == "original"
                    && fs::read_to_string(&r.recovery_path).unwrap() == "external")
        );
        fs::remove_file(&target).unwrap();
        assert!(matches!(
            write_file(temp.path(), "file.txt", "retained", "merged", false),
            Err(FileError::Changed(_))
        ));
        assert!(!target.exists());
        write_file(temp.path(), "file.txt", "retained", "merged", true).unwrap();
        assert_eq!(fs::read_to_string(target).unwrap(), "retained");
    }
    #[test]
    fn save_preserves_mode_and_independent_proposed_copy() {
        let temp = fixture();
        let target = temp.path().join("file.txt");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, fs::Permissions::from_mode(0o750)).unwrap();
        }
        write_file(temp.path(), "file.txt", "editor", "expected", false).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "editor");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&target).unwrap().permissions().mode() & 0o777,
                0o750
            );
        }
        fs::write(target, "later").unwrap();
        let records = records(temp.path()).unwrap();
        assert_eq!(records.len(), 3);
        let proposed = records.iter().find(|r| r.artifact == "proposed").unwrap();
        assert_eq!(
            fs::read_to_string(&proposed.recovery_path).unwrap(),
            "editor"
        );
        assert!(records.iter().all(|r| r.acknowledged));
    }
    #[test]
    fn changed_source_is_restored_without_overwrite_and_editor_proposal_survives() {
        let temp = fixture();
        let result = write_with_hook(
            temp.path(),
            "file.txt",
            "editor",
            "expected",
            false,
            |phase, source, _| {
                if phase == "temporary-ready" {
                    fs::write(source, "external")?;
                }
                Ok(())
            },
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("file changed on disk")
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("file.txt")).unwrap(),
            "external"
        );
        assert!(
            records(temp.path())
                .unwrap()
                .iter()
                .any(|r| r.artifact == "proposed"
                    && fs::read_to_string(&r.recovery_path).unwrap() == "editor")
        );
    }
    #[test]
    fn external_creation_after_hold_is_never_clobbered() {
        let temp = fixture();
        assert!(
            write_with_hook(
                temp.path(),
                "file.txt",
                "editor",
                "expected",
                false,
                |phase, _, target| {
                    if phase == "source-held" {
                        create_synced(target, b"external creation")?;
                    }
                    Ok(())
                }
            )
            .is_err()
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("file.txt")).unwrap(),
            "external creation"
        );
        let original = records(temp.path())
            .unwrap()
            .into_iter()
            .find(|r| r.artifact == "original")
            .unwrap();
        assert_eq!(
            fs::read_to_string(original.recovery_path).unwrap(),
            "expected"
        );
    }
    #[test]
    fn failed_install_restores_original_and_preserves_proposal() {
        let temp = fixture();
        assert!(
            write_with_hook(
                temp.path(),
                "file.txt",
                "editor",
                "expected",
                false,
                |phase, _, _| {
                    if phase == "held-validated" {
                        return Err(invalid("simulated failure"));
                    }
                    Ok(())
                }
            )
            .is_err()
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("file.txt")).unwrap(),
            "expected"
        );
        assert_eq!(records(temp.path()).unwrap().len(), 3);
    }
    #[test]
    fn interrupted_source_hold_recovers_after_restart_but_preserves_new_external_file() {
        let temp = fixture();
        let (directory, mut transaction) = create_transaction(temp.path(), "file.txt").unwrap();
        fs::rename(temp.path().join("file.txt"), directory.join("original")).unwrap();
        transaction.phase = "source-held".into();
        write_manifest(&directory, &transaction).unwrap();
        reconcile(temp.path()).unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("file.txt")).unwrap(),
            "expected"
        );
        fs::write(temp.path().join("file.txt"), "new external").unwrap();
        reconcile(temp.path()).unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("file.txt")).unwrap(),
            "new external"
        );
    }
    #[test]
    fn recovery_rejects_symlink_directories_and_does_not_follow_manifest_traversal() {
        let temp = fixture();
        let (directory, mut transaction) = create_transaction(temp.path(), "file.txt").unwrap();
        transaction.original_path = "../outside".into();
        write_manifest(&directory, &transaction).unwrap();
        assert!(records(temp.path()).unwrap().is_empty());
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let other = fixture();
            symlink(temp.path(), other.path().join(".git/vela-recovery")).unwrap();
            assert!(write_file(other.path(), "file.txt", "editor", "expected", false).is_err());
        }
    }
}
