import { execFile } from "node:child_process";

export function sqliteQuery(db: string, sql: string): Promise<Array<Record<string, unknown>>> {
  return new Promise((resolve) => {
    execFile("sqlite3", [db, "-json", sql], { timeout: 5000 }, (error, stdout) => {
      if (error) {
        resolve([]);
        return;
      }
      try {
        const parsed = JSON.parse(stdout.trim());
        resolve(Array.isArray(parsed) ? (parsed as Array<Record<string, unknown>>) : []);
      } catch {
        resolve([]);
      }
    });
  });
}