import Route from '@ember/routing/route';
import { service } from '@ember/service';

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
  @service sentry;

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

      return { crate, advisories, convertMarkdown, error: false };
    } catch (error) {
      this.sentry.captureException(error);
      return { crate, advisories: [], convertMarkdown: null, error: true };
    }
  }

  setupController(controller, { crate, advisories, convertMarkdown, error }) {
    super.setupController(...arguments);
    controller.crate = crate;
    controller.advisories = advisories;
    controller.convertMarkdown = convertMarkdown;
    controller.error = error;
  }
}
