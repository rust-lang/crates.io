import { resolve } from '$app/paths';
import { redirect } from '@sveltejs/kit';

export function load({ params }) {
  redirect(308, resolve('/crates/[crate_id]/settings', { crate_id: params.crate_id }));
}
