import { resolve } from '$app/paths';
import { redirect } from '@sveltejs/kit';

import { redirectTarget } from './redirect-target';

export async function load({ params, parent }) {
  let { crate, version, manifest } = await parent();

  // The selected file always lives in the URL (so the browser back button can
  // navigate between files). `/code` with no path redirects to the default
  // file, and a directory path redirects to the first file inside it.
  if (manifest) {
    let target = redirectTarget(manifest.files, params.path);
    if (target) {
      redirect(
        307,
        resolve('/crates/[crate_id]/[version_num]/code/[...path]', {
          crate_id: crate.id,
          version_num: version.num,
          path: target.path,
        }),
      );
    }
  }

  return { selectedPath: params.path || null };
}
