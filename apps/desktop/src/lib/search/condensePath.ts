/**
 * Deterministic shortened path for UI (full path string is unchanged on disk).
 * Keeps a head prefix, ellipsis, and tail (last 1–2 parent folders + filename).
 */

export function condensePath(fullPath: string, maxLen = 100): string {
  if (!fullPath || fullPath.length <= maxLen) return fullPath;

  const sep = fullPath.includes("\\") ? "\\" : "/";
  const norm = fullPath.replace(/\\/g, "/");
  const parts = norm.split("/").filter(Boolean);
  if (parts.length < 2) {
    return fullPath.length <= maxLen ? fullPath : fullPath.slice(0, maxLen - 3) + "...";
  }

  const fileName = parts[parts.length - 1]!;
  const dirs = parts.slice(0, -1);

  const headTargetChars = 44;
  const tailFolderCount = 2;

  const headDirs: string[] = [];
  let headLen = 0;
  for (const d of dirs) {
    const add = headDirs.length ? 1 + d.length : d.length;
    if (headLen + add > headTargetChars && headDirs.length > 0) break;
    headDirs.push(d);
    headLen += add;
  }

  const tailDirs =
    dirs.length >= tailFolderCount ? dirs.slice(-tailFolderCount) : dirs.slice();
  const tail = [...tailDirs, fileName].join(sep);

  const headEnd = headDirs.length;
  const tailStart = dirs.length - tailDirs.length;
  if (headEnd >= tailStart) {
    const half = Math.max(12, Math.floor((maxLen - 3) / 2));
    const take = Math.min(half, Math.floor(fullPath.length / 2));
    return fullPath.slice(0, take) + "..." + fullPath.slice(-take);
  }

  const head = headDirs.join(sep);
  let out = `${head}${sep}...${sep}${tail}`;
  if (out.length > maxLen) {
    out = `${head}${sep}...${sep}${fileName}`;
  }
  if (out.length > maxLen) {
    return fullPath.slice(0, maxLen - 3) + "...";
  }
  return out.replace(/\//g, sep);
}
