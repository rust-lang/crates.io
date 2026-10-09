import type { components } from '@crates-io/api-client';

import { error } from '@sveltejs/kit';

/** The result of an openapi-fetch request such as `client.GET()`. */
type ApiResult<T> =
  | { data: T; error?: never; response: Response }
  | { data?: never; error: components['schemas']['ApiErrorResponse']; response: Response };

/** The error page headings that `loadApi()` uses for failed requests. */
interface LoadApiOptions {
  /** The error page heading for any failure. */
  message: string;
  /** The error page heading for a `404 Not Found` response. */
  notFound?: string;
}

/**
 * Awaits a `client.GET()` request inside a SvelteKit load function and returns
 * its `data`, or throws a SvelteKit `error()` with the response status.
 */
export async function loadApi<T>(request: Promise<ApiResult<T>>, { message, notFound }: LoadApiOptions): Promise<T> {
  let result;
  try {
    result = await request;
  } catch {
    // Network errors are treated as `504 Gateway Timeout`
    error(504, message, { tryAgain: true });
  }

  let status = result.response.status;
  if (!result.response.ok || result.error) {
    if (status === 404 && notFound) {
      error(404, notFound);
    }

    let details = result.error?.errors?.[0]?.detail;
    error(status, message, { ...(details && { details }), tryAgain: isRetryableStatus(status) });
  }

  return result.data;
}

/** Returns whether a request that failed with `status` can succeed when it is retried. */
export function isRetryableStatus(status: number): boolean {
  return status >= 500 || status === 429;
}
