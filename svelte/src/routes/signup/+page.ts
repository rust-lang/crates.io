import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

/** Loads the details for an unfinished signup. */
export async function load({ fetch }) {
  let client = createClient({ fetch });
  let result;
  try {
    result = await client.GET('/api/private/session/signup');
  } catch {
    error(504, { message: 'Failed to load signup details', tryAgain: true });
  }

  if (result.error) {
    let status = result.response.status;
    error(status, {
      message: result.error.errors[0]?.detail ?? 'Failed to load signup details',
      ...(status >= 500 && { tryAgain: true }),
    });
  }

  return { signup: result.data.signup };
}
