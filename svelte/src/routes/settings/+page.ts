import { resolve } from '$app/paths';
import { redirect } from '@sveltejs/kit';

export function load() {
  redirect(302, resolve('/settings/profile'));
}
