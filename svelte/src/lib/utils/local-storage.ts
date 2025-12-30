/**
 * Wrappers around localStorage that silently handle errors.
 *
 * localStorage can throw in certain situations:
 * - Safari private browsing mode
 * - Storage quota exceeded
 * - Sandboxed iframes with restricted storage access
 */

export function getItem(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function setItem(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // ignored
  }
}

export function removeItem(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    // ignored
  }
}
