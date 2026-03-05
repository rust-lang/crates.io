import { resolve } from '$app/paths';
import { redirect } from '@sveltejs/kit';

// `cargo login` shows links to https://crates.io/me to access API tokens,
// so this route redirects the user to the API tokens settings page.
export function load() {
  redirect(308, resolve('/settings/tokens'));
}
