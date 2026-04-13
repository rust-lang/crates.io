/**
 * Normalizes a `rust-version` string for display. If the value has only two
 * version components (e.g. `1.69`), a `.0` patch component is appended to
 * produce a full semver-shaped string (`1.69.0`). All other inputs are
 * returned unchanged.
 */
export function normalizeMsrv(rustVersion: string): string {
  return /^[^.]+\.[^.]+$/.test(rustVersion) ? `${rustVersion}.0` : rustVersion;
}
