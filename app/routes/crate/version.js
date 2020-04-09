import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import prerelease from 'semver/functions/prerelease';

export default Route.extend({
  flashMessages: service(),

  async model(params) {
    const requestedVersion = params.version_num;
    const crate = this.modelFor('crate');
    const controller = this.controllerFor(this.routeName);
    const maxVersion = crate.get('max_version');

    let versions = await crate.get('versions');

    const isUnstableVersion = version => !!prerelease(version);

    // Fallback to the crate's last stable version
    // If `max_version` is `0.0.0` then all versions have been yanked
    if (!params.version_num && maxVersion !== '0.0.0') {
      if (isUnstableVersion(maxVersion)) {
        const latestStableVersion = versions.find(version => {
          // Find the latest version that is stable AND not-yanked.
          if (!isUnstableVersion(version.get('num')) && !version.get('yanked')) {
            return version;
          }
        });

        if (latestStableVersion == null) {
          // Cannot find any version that is stable AND not-yanked.
          // The fact that "maxVersion" itself cannot be found means that
          // we have to fall back to the latest one that is unstable....
          const latestUnyankedVersion = versions.find(version => {
            // Find the latest version that is stable AND not-yanked.
            if (!version.get('yanked')) {
              return version;
            }
          });

          if (latestStableVersion == null) {
            // There's not even any unyanked version...
            params.version_num = maxVersion;
          } else {
            params.version_num = latestUnyankedVersion;
          }
        } else {
          params.version_num = latestStableVersion.get('num');
        }
      } else {
        params.version_num = maxVersion;
      }
    }

    controller.set('crate', crate);
    controller.set('requestedVersion', requestedVersion);

    const version = versions.find(version => version.get('num') === params.version_num);
    if (params.version_num && !version) {
      this.flashMessages.queue(`Version '${params.version_num}' of crate '${crate.get('name')}' does not exist`);
    }

    return version || versions.find(version => version.get('num') === maxVersion) || versions.objectAt(0);
  },

  setupController(controller) {
    this._super(...arguments);
    controller.loadReadmeTask.perform();

    let { crate } = controller;
    if (!crate.documentation || crate.documentation.startsWith('https://docs.rs/')) {
      controller.loadDocsBuilds.perform();
    }
  },

  serialize(model) {
    let version_num = model.get('num');
    return { version_num };
  },
});
