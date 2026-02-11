import { redirect } from '@sveltejs/kit';

export function load() {
  redirect(301, 'https://doc.rust-lang.org/cargo/getting-started/installation.html');
}
