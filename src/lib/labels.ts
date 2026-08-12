export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function shortPath(path: string, workspacePath: string): string {
  if (path.startsWith(workspacePath)) {
    return path.replace(workspacePath, ".");
  }
  const homePrefix = path.match(/^\/Users\/[^/]+/)?.[0];
  return homePrefix ? path.replace(homePrefix, "~") : path;
}
