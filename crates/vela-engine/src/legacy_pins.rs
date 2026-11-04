//! Read-only recovery of Source's renderer-local pinned session ids.
//!
//! Source keeps pins in Chromium localStorage (`vela.pinnedSessions`); there is
//! no profile JSON for them. Chromium persists localStorage in a LevelDB
//! directory. We reconstruct the newest value from the write-ahead `.log`
//! files (LevelDB record framing plus `WriteBatch` entries) and fall back to a
//! bounded raw scan of compacted `.ldb` tables so a long-lived profile still
//! migrates after compaction. Nothing in the source profile is written.

use std::{fs, path::Path};

const KEY: &[u8] = b"vela.pinnedSessions";
const BLOCK: usize = 32 * 1024;
const RAW_WINDOW: usize = 512;

/// `None` means the profile has no readable localStorage store (so the caller
/// should retry on a later launch). `Some` means the store was scanned; the
/// vector may legitimately be empty.
pub(crate) fn read(profile: &Path) -> Option<Vec<String>> {
    let directory = profile.join("Local Storage").join("leveldb");
    let entries = fs::read_dir(&directory).ok()?;
    let mut logs = Vec::new();
    let mut tables = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(stem) = name
            .strip_suffix(".log")
            .or_else(|| name.strip_suffix(".ldb"))
        else {
            continue;
        };
        let Ok(sequence) = stem.parse::<u64>() else {
            continue;
        };
        if name.ends_with(".log") {
            logs.push((sequence, path));
        } else {
            tables.push((sequence, path));
        }
    }
    logs.sort_by_key(|(sequence, _)| *sequence);
    tables.sort_by_key(|(sequence, _)| *sequence);

    let mut latest_sequence = 0;
    let mut latest = None;
    let mut found = false;
    for (_, path) in logs {
        scan_log(&path, &mut latest_sequence, &mut latest, &mut found);
    }
    if found {
        return Some(latest.flatten().unwrap_or_default());
    }
    for (_, path) in tables {
        if let Some(pins) = scan_table(&path) {
            latest = Some(Some(pins));
        }
    }
    Some(latest.flatten().unwrap_or_default())
}

fn scan_log(
    path: &Path,
    latest_sequence: &mut u64,
    latest: &mut Option<Option<Vec<String>>>,
    found: &mut bool,
) {
    let Ok(bytes) = fs::read(path) else {
        return;
    };
    let mut partial = Vec::new();
    let mut offset = 0;
    while offset + 7 <= bytes.len() {
        let block_remaining = BLOCK - (offset % BLOCK);
        if block_remaining < 7 {
            offset += block_remaining;
            continue;
        }
        let length = u16::from_le_bytes([bytes[offset + 4], bytes[offset + 5]]) as usize;
        let kind = bytes[offset + 6];
        offset += 7;
        if length == 0 && kind == 0 {
            // Zero padding fills the rest of this block.
            offset += block_remaining - 7;
            continue;
        }
        if offset + length > bytes.len() {
            break;
        }
        let payload = &bytes[offset..offset + length];
        offset += length;
        match kind {
            1 => apply_batch(payload, latest_sequence, latest, found),
            2 => {
                partial.clear();
                partial.extend_from_slice(payload);
            }
            3 => partial.extend_from_slice(payload),
            4 => {
                partial.extend_from_slice(payload);
                apply_batch(&partial, latest_sequence, latest, found);
                partial.clear();
            }
            _ => partial.clear(),
        }
    }
}

fn apply_batch(
    record: &[u8],
    latest_sequence: &mut u64,
    latest: &mut Option<Option<Vec<String>>>,
    found: &mut bool,
) {
    if record.len() < 12 {
        return;
    }
    let base = u64::from_le_bytes(record[..8].try_into().unwrap_or_default());
    let count = u32::from_le_bytes(record[8..12].try_into().unwrap_or_default());
    if count > 1_000_000 {
        return;
    }
    let mut cursor = 12;
    for index in 0..count {
        let Some(&operation) = record.get(cursor) else {
            return;
        };
        cursor += 1;
        let Some(key_length) = varint(record, &mut cursor) else {
            return;
        };
        let Some(key) = record.get(cursor..cursor.saturating_add(key_length)) else {
            return;
        };
        cursor += key_length;
        if !key.ends_with(KEY) {
            if operation == 1 {
                let Some(value_length) = varint(record, &mut cursor) else {
                    return;
                };
                cursor = cursor.saturating_add(value_length);
            }
            continue;
        }
        let sequence = base.saturating_add(u64::from(index));
        match operation {
            0 => store(latest_sequence, latest, found, sequence, None),
            1 => {
                let Some(value_length) = varint(record, &mut cursor) else {
                    return;
                };
                let Some(value) = record.get(cursor..cursor.saturating_add(value_length)) else {
                    return;
                };
                cursor += value_length;
                store(
                    latest_sequence,
                    latest,
                    found,
                    sequence,
                    decode_value(value),
                );
            }
            _ => return,
        }
    }
}

fn store(
    latest_sequence: &mut u64,
    latest: &mut Option<Option<Vec<String>>>,
    found: &mut bool,
    sequence: u64,
    value: Option<Vec<String>>,
) {
    if *found && sequence < *latest_sequence {
        return;
    }
    *found = true;
    *latest_sequence = sequence;
    *latest = Some(value);
}

fn varint(bytes: &[u8], cursor: &mut usize) -> Option<usize> {
    let mut value = 0usize;
    let mut shift = 0;
    loop {
        let byte = *bytes.get(*cursor)?;
        *cursor += 1;
        value = value.checked_add(((byte & 0x7f) as usize).checked_shl(shift)?)?;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift = shift.checked_add(7)?;
        if shift >= usize::BITS {
            return None;
        }
    }
}

fn decode_value(value: &[u8]) -> Option<Vec<String>> {
    let (&encoding, body) = value.split_first()?;
    let text = match encoding {
        0 => {
            let body = body.strip_suffix(&[0, 0]).unwrap_or(body);
            if body.len() % 2 != 0 {
                return None;
            }
            let units = body
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_le_bytes(*pair))
                .collect::<Vec<_>>();
            String::from_utf16_lossy(&units)
        }
        1 => body.iter().map(|&byte| byte as char).collect(),
        _ => String::from_utf8(body.to_vec()).ok()?,
    };
    parse_pins(&text)
}

fn parse_pins(text: &str) -> Option<Vec<String>> {
    let pins = serde_json::from_str::<Vec<String>>(text.trim_end_matches('\0')).ok()?;
    Some(
        pins.into_iter()
            .filter(|pin| !pin.is_empty())
            .collect::<Vec<_>>(),
    )
}

/// Compacted tables are Snappy-framed, but literals survive byte-for-byte.
/// Locate the key and try the bounded JSON arrays that follow it; the first
/// valid value is the newest because LevelDB orders equal keys by sequence.
fn scan_table(path: &Path) -> Option<Vec<String>> {
    let bytes = fs::read(path).ok()?;
    let mut search = 0;
    let mut found = None;
    while let Some(relative) = find(&bytes[search..], KEY) {
        let after = search + relative + KEY.len();
        let window = &bytes[after..bytes.len().min(after + RAW_WINDOW)];
        if let Some(start) = window.iter().position(|byte| *byte == b'[') {
            for end in start..window.len() {
                if window[end] != b']' {
                    continue;
                }
                if let Ok(text) = std::str::from_utf8(&window[start..=end])
                    && let Some(pins) = parse_pins(text)
                {
                    found = Some(pins);
                    break;
                }
            }
        }
        search = after;
    }
    found
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn batch(sequence: u64, entries: &[(u8, &[u8], &[u8])]) -> Vec<u8> {
        let mut record = Vec::new();
        record.extend_from_slice(&sequence.to_le_bytes());
        record.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (operation, key, value) in entries {
            record.push(*operation);
            put_varint(&mut record, key.len());
            record.extend_from_slice(key);
            if *operation == 1 {
                put_varint(&mut record, value.len());
                record.extend_from_slice(value);
            }
        }
        record
    }

    fn put_varint(out: &mut Vec<u8>, mut value: usize) {
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if value == 0 {
                return;
            }
        }
    }

    fn append_record(out: &mut Vec<u8>, kind: u8, payload: &[u8]) {
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        out.push(kind);
        out.extend_from_slice(payload);
    }

    fn latin1(value: &str) -> Vec<u8> {
        let mut encoded = vec![1];
        encoded.extend_from_slice(value.as_bytes());
        encoded
    }

    fn utf16(value: &str) -> Vec<u8> {
        let mut encoded = vec![0];
        for unit in value.encode_utf16() {
            encoded.extend_from_slice(&unit.to_le_bytes());
        }
        encoded
    }

    fn profile(log: &[u8]) -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("Local Storage").join("leveldb");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("000003.log"), log).unwrap();
        let profile = temp.path().to_path_buf();
        (temp, profile)
    }

    #[test]
    fn newest_batch_wins_and_decodes_both_chromium_encodings() {
        let mut log = Vec::new();
        append_record(
            &mut log,
            1,
            &batch(
                1,
                &[(
                    1,
                    b"_file://\x00\x01vela.pinnedSessions",
                    &latin1(r#"["ses_one"]"#),
                )],
            ),
        );
        append_record(
            &mut log,
            1,
            &batch(
                9,
                &[(
                    1,
                    b"_file://\x00\x01vela.pinnedSessions",
                    &utf16(r#"["ses_one", "ses_界"]"#),
                )],
            ),
        );
        let (_temp, profile) = profile(&log);
        assert_eq!(
            read(&profile),
            Some(vec!["ses_one".into(), "ses_界".into()])
        );
    }

    #[test]
    fn later_delete_clears_pins_and_fragmented_records_reassemble() {
        let value = latin1(r#"["ses_a","ses_b"]"#);
        let record = batch(
            3,
            &[(
                1,
                b"_file://\x00\x01vela.pinnedSessions",
                value.as_slice(),
            )],
        );
        let split = record.len() / 2;
        let mut log = Vec::new();
        append_record(&mut log, 2, &record[..split]);
        append_record(&mut log, 4, &record[split..]);
        append_record(
            &mut log,
            1,
            &batch(
                5,
                &[(0, b"_file://\x00\x01vela.pinnedSessions", b"")],
            ),
        );
        let (_temp, profile) = profile(&log);
        assert_eq!(read(&profile), Some(Vec::new()));
    }

    #[test]
    fn compacted_table_fallback_scans_a_bounded_json_array() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("Local Storage").join("leveldb");
        fs::create_dir_all(&directory).unwrap();
        let mut table = b"\x01snappy-literal".to_vec();
        table.extend_from_slice(b"_file://\x00\x01vela.pinnedSessions");
        table.extend_from_slice(br#"["ses_table","ses_other"]trailing"#);
        fs::write(directory.join("000007.ldb"), table).unwrap();
        assert_eq!(
            read(temp.path()),
            Some(vec!["ses_table".into(), "ses_other".into()])
        );
    }

    #[test]
    fn missing_store_is_distinct_from_an_empty_scan() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(read(temp.path()), None);
        let directory = temp.path().join("Local Storage").join("leveldb");
        fs::create_dir_all(directory).unwrap();
        assert_eq!(read(temp.path()), Some(Vec::new()));
    }

    #[test]
    fn malformed_or_truncated_records_are_ignored() {
        let mut log = Vec::new();
        append_record(&mut log, 1, b"short");
        append_record(&mut log, 1, &[9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9]);
        log.extend_from_slice(&[0, 0, 0, 0, 200, 0, 1, 1]);
        let (_temp, profile) = profile(&log);
        assert_eq!(read(&profile), Some(Vec::new()));
    }
}
