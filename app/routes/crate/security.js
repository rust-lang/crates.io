import Route from '@ember/routing/route';
import { service } from '@ember/service';

import { versionRanges } from 'crates-io/utils/version-ranges';

function extractCvss(advisory) {
  // Prefer V4 over V3
  let cvssEntry =
    advisory.severity?.find(s => s.type === 'CVSS_V4') ?? advisory.severity?.find(s => s.type === 'CVSS_V3');
  return cvssEntry?.score ?? null;
}

async function fetchAdvisories(crateId) {
  let url = `https://rustsec.org/packages/${crateId}.json`;
  let response = await fetch(url);
  if (response.status === 404) {
    return [];
  } else if (response.ok) {
    let advisories = await response.json();
    return advisories
      .filter(
        advisory =>
          !advisory.withdrawn &&
          !advisory.affected?.some(affected => affected.database_specific?.informational === 'unmaintained'),
      )
      .map(advisory => ({
        ...advisory,
        versionRanges: versionRanges(advisory),
        cvss: extractCvss(advisory),
      }));
  } else {
    throw new Error(`HTTP error! status: ${response}`);
  }
}

export default class SecurityRoute extends Route {
  @service router;
  @service sentry;

  async model(_params, transition) {
    let crate = this.modelFor('crate');
    try {
      let [advisories, micromarkModule, gfmModule] = await Promise.all([
        fetchAdvisories(crate.id),
        import('micromark'),
        import('micromark-extension-gfm'),
      ]);

      let convertMarkdown = markdown => {
        return micromarkModule.micromark(markdown, {
          extensions: [gfmModule.gfm()],
          htmlExtensions: [gfmModule.gfmHtml()],
        });
      };

      return { crate, advisories, convertMarkdown };
    } catch (error) {
      let title = `${crate.name}: Failed to load advisories`;
      return this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
    }
  }

  setupController(controller, { crate, advisories, convertMarkdown }) {
    super.setupController(...arguments);
    controller.crate = crate;
    controller.advisories = advisories;
    controller.convertMarkdown = convertMarkdown;
  }
}
