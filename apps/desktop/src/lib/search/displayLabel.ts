/**
 * UI-only display label derived from filename (and optionally path context).
 * Does not rename files; Open/Copy still use real paths from SearchHit.
 */

export function fileNameFromFullPath(fullPath: string): string {
  const n = fullPath.replace(/\\/g, "/");
  const i = n.lastIndexOf("/");
  return i >= 0 ? n.slice(i + 1) : fullPath;
}

function truncate(s: string, max: number): string {
  if (s.length <= max) return s;
  return s.slice(0, max - 1) + "…";
}

/**
 * Heuristic, deterministic label. Falls back to the original filename when no rule applies.
 */
export function smartDisplayLabel(fileName: string, fullPath: string): string {
  if (!fileName.trim()) return fileName;

  const pathNorm = fullPath.replace(/\\/g, "/").toLowerCase();
  const lowerName = fileName.toLowerCase();

  const dot = fileName.lastIndexOf(".");
  const base = dot > 0 ? fileName.slice(0, dot) : fileName;
  const ext = dot > 0 ? fileName.slice(dot + 1).toLowerCase() : "";

  // High-confidence path + name patterns (no file reads)
  if (lowerName === "index.txt" && pathNorm.includes("edge") && pathNorm.includes("cachestorage")) {
    return "Edge Cache Index";
  }

  if (ext === "pdf" && /^[A-Z]{2,5}$/.test(base)) {
    return `${base} Document`;
  }

  if (base.length <= 3 && ext === "pdf") {
    return `${base} Document`;
  }

  // Filename cleanup: strip leading noise, trailing dates, normalize separators
  let s = base;
  s = s.replace(/^\d+[-\s]*(?:MFD\s*)?/i, "");
  s = s.replace(/\s*[-–—]?\s*\d{1,2}[-_/]\d{1,2}[-_/]\d{2,4}\s*$/i, "");
  s = s.replace(/\s*[-–—]?\s*Navi-Planner\s+\d{1,2}[-_/]\d{1,2}[-_/]\d{2,4}\s*$/i, "");
  s = s.replace(/[-_]{2,}/g, " ");
  s = s.replace(/\s+/g, " ").trim();

  // Drop trailing single-letter version noise sometimes left behind
  s = s.replace(/\s+[-_]?\s*[vV]?\d+$/i, "").trim();

  if (s.length < 2) return fileName;

  return truncate(s, 96);
}
