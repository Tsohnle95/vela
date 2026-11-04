export function migrateVelaStorage(storage: Storage): void {
  const newPrefix = "vela";
  const oldPrefixes = [
    String.fromCharCode(109, 105, 114, 97),
    String.fromCharCode(111, 114, 98, 105, 116),
    String.fromCharCode(111, 112, 101, 110, 115, 104, 101, 108, 108)
  ];
  const oldStorageKeys = oldPrefixes.flatMap((prefix) =>
    Array.from({ length: storage.length }, (_, index) => storage.key(index))
      .filter((key): key is string => key?.startsWith(`${prefix}.`) ?? false)
      .map((key) => ({ key, prefix }))
  );

  for (const { key: oldKey, prefix } of oldStorageKeys) {
    const newKey = `${newPrefix}${oldKey.slice(prefix.length)}`;
    if (storage.getItem(newKey) === null) {
      const value = storage.getItem(oldKey);
      if (value !== null) storage.setItem(newKey, value);
    }
    storage.removeItem(oldKey);
  }
}
