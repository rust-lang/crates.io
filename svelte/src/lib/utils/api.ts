/** Returns whether a request that failed with `status` can succeed when it is retried. */
export function isRetryableStatus(status: number): boolean {
  return status >= 500 || status === 429;
}
