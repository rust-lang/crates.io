import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import * as Sentry from '@sentry/browser';
import prerelease from 'semver/functions/prerelease';

import { AjaxError } from '../../utils/ajax';

function isUnstableVersion(version) {
  return !!prerelease(version);
}

export default class VersionRoute extends Route {
  @service notifications;

  async model(params) {
    const requestedVersion = params.version_num;
    const crate = this.modelFor('crate');
    const maxVersion = crate.max_version;

    let versions = await crate.get('versions');

    // Fallback to the crate's last stable version
    // If `max_version` is `0.0.0` then all versions have been yanked
    if (!params.version_num && maxVersion !== '0.0.0') {
      if (isUnstableVersion(maxVersion)) {
        // Find the latest version that is stable AND not-yanked.
        const latestStableVersion = versions.find(version => !isUnstableVersion(version.num) && !version.yanked);

        if (latestStableVersion == null) {
          // Cannot find any version that is stable AND not-yanked.
          // The fact that "maxVersion" itself cannot be found means that
          // we have to fall back to the latest one that is unstable....

          // Find the latest version that not yanked.
          const latestUnyankedVersion = versions.find(version => !version.yanked);

          if (latestUnyankedVersion == null) {
            // There's not even any unyanked version...
            params.version_num = maxVersion;
          } else {
            params.version_num = latestUnyankedVersion.num;
          }
        } else {
          params.version_num = latestStableVersion.num;
        }
      } else {
        params.version_num = maxVersion;
      }
    }

    const version = versions.find(version => version.num === params.version_num);
    if (params.version_num && !version) {
      this.notifications.error(`Version '${params.version_num}' of crate '${crate.name}' does not exist`);
    }

    return {
      crate,
      requestedVersion,
      version: version || versions.find(version => version.num === maxVersion) || versions.objectAt(0),
    };
  }

  setupController(controller, model) {
    super.setupController(...arguments);

    model.version.loadDepsTask.perform();
    if (!model.version.authorNames) {
      model.version.loadAuthorsTask.perform();
    }

    controller.loadReadmeTask.perform().catch(() => {
      // ignored
    });

    let { crate } = model;
    if (!crate.documentation || crate.documentation.startsWith('https://docs.rs/')) {
      controller.loadDocsBuildsTask.perform().catch(error => {
        // report unexpected errors to Sentry and ignore `ajax()` errors
        if (!(error instanceof AjaxError)) {
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
