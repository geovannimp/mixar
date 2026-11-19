/// Comparable form of a filesystem path: forward slashes, no trailing slash.
///
/// The library stores collection folders canonicalized, while the drive
/// browser hands out paths as the platform reports them, so both sides go
/// through this before they are compared.
String normalizeFolderPath(String path) {
  var normalized = path.trim().replaceAll(r'\', '/');
  while (normalized.length > 1 && normalized.endsWith('/')) {
    normalized = normalized.substring(0, normalized.length - 1);
  }
  return normalized;
}

/// The name a folder is known by in the sidebar: its last path segment.
///
/// A volume root keeps the volume's own name from the caller; this only reads
/// the path, so it is the fallback for any folder below a volume.
String folderDisplayName(String path) {
  final normalized = normalizeFolderPath(path);
  final separator = normalized.lastIndexOf('/');
  if (separator < 0) {
    return normalized;
  }
  final name = normalized.substring(separator + 1);
  return name.isEmpty ? normalized : name;
}
