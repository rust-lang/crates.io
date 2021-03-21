import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import * as Sentry from '@sentry/browser';
import { didCancel } from 'ember-concurrency';

import { AjaxError } from '../../utils/ajax';

export default class VersionRoute extends Route {
  @service notifications;

  async model(params) {
    let crate = this.modelFor('crate');

    let versions;
    try {
      versions = await crate.get('versions');
    } catch {
      this.notifications.error(`Loading data for the '${crate.name}' crate failed. Please try again later!`);
      this.replaceWith('index');
      return;
    }

    let version;
    let requestedVersion = params.version_num;
    if (requestedVersion) {
      version = versions.find(version => version.num === requestedVersion);
      if (!version) {
        this.notifications.error(`Version '${requestedVersion}' of crate '${crate.name}' does not exist`);
        this.replaceWith('crate.index');
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
          Sentry.captureException(error);
        }
      });
    }
  }

  serialize(model) {
    let version_num = model.num;
    return { version_num };
  }
}
