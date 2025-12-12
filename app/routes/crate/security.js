import Route from '@ember/routing/route';

import { didCancel } from 'ember-concurrency';

import { AjaxError } from '../../utils/ajax';

async function fetchAdvisories(crateId) {
  let url = `https://rustsec.org/packages/${crateId}.json`;
  let response = await fetch(url);
  if (response.status === 404) {
    return [];
  } else if (response.ok) {
    return await response.json();
  } else {
    throw new Error(`HTTP error! status: ${response}`);
  }
}

export default class SecurityRoute extends Route {
  queryParams = {
    sort: { refreshModel: true },
  };

  async model() {
    let crate = this.modelFor('crate');
    try {
      let [advisories, micromarkModule, gfmModule] = await Promise.all([
        fetchAdvisories(crate.id),
        import('micromark'),
        import('micromark-extension-gfm'),
      ]);

      const convertMarkdown = markdown => {
        return micromarkModule.micromark(markdown, {
          extensions: [gfmModule.gfm()],
          htmlExtensions: [gfmModule.gfmHtml()],
        });
      };

      return { crate, advisories, convertMarkdown };
    } catch (error) {
      // report unexpected errors to Sentry and ignore `ajax()` errors
      if (!didCancel(error) && !(error instanceof AjaxError)) {
        this.sentry.captureException(error);
      }
    }
  }

  setupController(controller, { crate, advisories, convertMarkdown }) {
    super.setupController(...arguments);
    // reset when crate changes
    if (crate && crate !== controller.crate) {
      controller.reset();
    }
    controller.crate = crate;
    controller.advisories = advisories;
    controller.convertMarkdown = convertMarkdown;
  }
}
