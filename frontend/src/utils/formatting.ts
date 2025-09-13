/**
 * Format duration from milliseconds to human readable string
 * @param durationMs Duration in milliseconds
 * @returns Formatted duration string (e.g., "1.2s", "45ms", "2.1m")
 */
export function formatDuration(durationMs: number | null): string {
  if (!durationMs) return 'N/A'
  
  if (durationMs < 1000) return `${durationMs}ms`
  if (durationMs < 60000) return `${(durationMs / 1000).toFixed(1)}s`
  return `${(durationMs / 60000).toFixed(1)}m`
}

/**
 * Format timestamp to readable date string
 * @param timestamp Timestamp in microseconds or milliseconds
 * @returns Formatted date string
 */
export function formatTimestamp(timestamp: number): string {
  // Handle microsecond timestamps (convert to milliseconds)
  const ms = timestamp > 1e12 ? timestamp / 1000 : timestamp
  const date = new Date(ms)
  
  if (isNaN(date.getTime())) return 'Invalid Date'
  
  return date.toLocaleDateString() + ' ' + date.toLocaleTimeString()
}

/**
 * Format bytes to human readable string
 * @param bytes Number of bytes
 * @returns Formatted string (e.g., "1.2 KB", "3.4 MB")
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}