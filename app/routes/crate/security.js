import Route from '@ember/routing/route';
import { service } from '@ember/service';

import { loadCvssModule, parseCvss } from 'crates-io/utils/cvss';
import { versionRanges } from 'crates-io/utils/version-ranges';

async function extractCvssWithScore(advisory) {
  // Prefer V4 over V3
  let cvssEntry =
    advisory.severity?.find(s => s.type === 'CVSS_V4') ?? advisory.severity?.find(s => s.type === 'CVSS_V3');

  if (!cvssEntry?.score) {
    return null;
  }

  // Parse the vector using WASM module to get calculated score and severity
  try {
    let parsed = await parseCvss(cvssEntry.score);
    return {
      vector: cvssEntry.score,
      calculatedScore: parsed.score,
      severity: parsed.severity,
      version: parsed.version,
      valid: parsed.valid,
    };
  } catch {
    // Fallback to just returning the vector string
    return {
      vector: cvssEntry.score,
      calculatedScore: null,
      severity: null,
      version: null,
      valid: false,
    };
  }
}

async function fetchAdvisories(crateId) {
  let url = `https://rustsec.org/packages/${crateId}.json`;
  let response = await fetch(url);
  if (response.status === 404) {
    return [];
  } else if (response.ok) {
    let advisories = await response.json();

    // Filter advisories
    let filtered = advisories.filter(
      advisory =>
        !advisory.withdrawn &&
        !advisory.affected?.some(affected => affected.database_specific?.informational === 'unmaintained'),
    );

    // Process CVSS scores in parallel
    return Promise.all(
      filtered.map(async advisory => ({
        ...advisory,
        versionRanges: versionRanges(advisory),
        cvss: await extractCvssWithScore(advisory),
      })),
    );
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
        loadCvssModule(), // Pre-load WASM module
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
