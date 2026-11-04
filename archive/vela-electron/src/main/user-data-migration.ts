import { existsSync, renameSync } from "node:fs";

export function migrateUserDataDirectory(
  currentPath: string,
  previousPaths: readonly string[],
  customPath: boolean,
  exists: (path: string) => boolean = existsSync,
  rename: (from: string, to: string) => void = renameSync
): string | null {
  if (customPath || exists(currentPath)) return null;
  const previousPath = previousPaths.find(exists);
  if (!previousPath) return null;
  try {
    rename(previousPath, currentPath);
    return null;
  } catch {
    return previousPath;
  }
}
