import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import { didCancel } from 'ember-concurrency';

import { AjaxError } from '../../utils/ajax';

export default class VersionRoute extends Route {
  @service router;
  @service sentry;

  async model(params, transition) {
    let crate = this.modelFor('crate');

    let versions;
    try {
      versions = await crate.get('versions');
    } catch (error) {
      return this.router.replaceWith('catch-all', { transition, error, title: 'Crate failed to load', tryAgain: true });
    }

    let version;
    let requestedVersion = params.version_num;
    if (requestedVersion) {
      version = versions.find(version => version.num === requestedVersion);
      if (!version) {
        return this.router.replaceWith('catch-all', { transition, title: 'Version not found' });
      }
    } else {
      let { defaultVersion } = crate;
      version = versions.find(version => version.num === defaultVersion) ?? versions.lastObject;
    }

    return { crate, requestedVersion, version };
  }

  setupController(controller, model) {
    super.setupController(...arguments);

    controller.loadReadmeTask.perform().catch(() => {
      // ignored
    });

    let { crate, version } = model;
    if (!crate.documentation || crate.documentation.startsWith('https://docs.rs/')) {
      version.loadDocsBuildsTask.perform().catch(error => {
        // report unexpected errors to Sentry and ignore `ajax()` errors
        if (!didCancel(error) && !(error instanceof AjaxError)) {
          this.sentry.captureException(error);
        }
      });
    }
  }

  serialize(model) {
    let version_num = model.num;
    return { version_num };
  }
}
