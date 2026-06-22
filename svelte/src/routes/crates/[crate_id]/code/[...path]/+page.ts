import { resolve } from '$app/paths';
import { redirect } from '@sveltejs/kit';

export async function load({ parent, params }) {
  let { defaultVersion } = await parent();
  let crate_id = params.crate_id;
  let version_num = defaultVersion.num;
  let path = params.path;
  redirect(307, resolve('/crates/[crate_id]/[version_num]/code/[...path]', { crate_id, version_num, path }));
}
