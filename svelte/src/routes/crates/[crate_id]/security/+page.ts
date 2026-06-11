import { error } from '@sveltejs/kit';

import { loadAdvisories } from '$lib/utils/rustsec';

export async function load({ fetch, params }) {
  let crateName = params.crate_id;

  try {
    let [advisories, micromarkModule, gfmModule] = await Promise.all([
      loadAdvisories(fetch, crateName),
      import('micromark'),
      import('micromark-extension-gfm'),
    ]);

    let convertMarkdown = (markdown: string): string => {
      return micromarkModule.micromark(markdown, {
        extensions: [gfmModule.gfm()],
        htmlExtensions: [gfmModule.gfmHtml()],
      });
    };

    return { advisories, convertMarkdown };
  } catch {
    error(500, { message: `${crateName}: Failed to load advisories`, tryAgain: true });
  }
}
