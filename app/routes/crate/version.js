import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import { waitForPromise } from '@ember/test-waiters';

import { didCancel } from 'ember-concurrency';
import semverSort from 'semver/functions/rsort';

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
      let title = `${crate.name}: Failed to load version data`;
      return this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
    }

    let version;
    let requestedVersion = params.version_num;
    if (requestedVersion) {
      version = versions.find(version => version.num === requestedVersion);
      if (!version) {
        let title = `${crate.name}: Version ${requestedVersion} not found`;
        return this.router.replaceWith('catch-all', { transition, title });
      }
    } else {
      let { default_version } = crate;
      version = versions.find(version => version.num === default_version);

      if (!version) {
        let versionNums = versions.map(it => it.num);
        semverSort(versionNums, { loose: true });

        version = versions.find(version => version.num === versionNums[0]);
      }
    }

    return { crate, requestedVersion, version };
  }

  setupController(controller, model) {
    super.setupController(...arguments);

    waitForPromise(controller.loadReadmeTask.perform()).catch(() => {
      // ignored
    });
    waitForPromise(controller.loadDownloadsTask.perform()).catch(() => {
      // ignored
    });

    let { crate, version } = model;
    if (!crate.documentation || crate.documentation.startsWith('https://docs.rs/')) {
      version.loadDocsStatusTask.perform().catch(error => {
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
