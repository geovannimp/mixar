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
