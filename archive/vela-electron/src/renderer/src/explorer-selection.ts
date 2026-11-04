export function compactExplorerSelection(paths: readonly string[]): string[] {
  const unique = [...new Set(paths.filter((path) => path.length > 0))];
  return unique.filter((path) => !unique.some((ancestor) =>
    ancestor !== path && path.startsWith(`${ancestor}/`)
  ));
}

export function movableExplorerPaths(paths: readonly string[], destination: string): string[] {
  return compactExplorerSelection(paths).filter((path) => {
    if (path === destination || destination.startsWith(`${path}/`)) return false;
    const parent = path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
    return parent !== destination;
  });
}
