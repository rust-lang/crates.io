import { observer } from '@ember/object';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import fetch from 'fetch';
import ajax from 'ember-fetch/ajax';

export default Route.extend({
    session: service(),

    flashMessages: service(),

    refreshAfterLogin: observer('session.isLoggedIn', function() {
        this.refresh();
    }),

    async model(params) {
        const requestedVersion = params.version_num === 'all' ? '' : params.version_num;
        const crate = this.modelFor('crate');
        const controller = this.controllerFor(this.routeName);
        const maxVersion = crate.get('max_version');

        const isUnstableVersion = version => {
            const versionLen = version.length;
            let majorMinorPatchChars = 0;
            let result = false;

            for (let i = 0; i < versionLen; i++) {
                const char = version.charAt(i);

                if (!isNaN(parseInt(char)) || char === '.') {
                    majorMinorPatchChars++;
                } else {
                    break;
                }
            }

            if (versionLen !== majorMinorPatchChars) {
                result = true;
            }

            return result;
        };

        const fetchCrateDocumentation = () => {
            if (!crate.get('documentation') || crate.get('documentation').substr(0, 16) === 'https://docs.rs/') {
                let crateName = crate.get('name');
                let crateVersion = params.version_num;
                ajax(`https://docs.rs/crate/${crateName}/${crateVersion}/builds.json`, { mode: 'cors' }).then(r => {
                    if (r.length > 0 && r[0].build_status === true) {
                        crate.set('documentation', `https://docs.rs/${crateName}/${crateVersion}/`);
                    }
                });
            }
        };

        // Fallback to the crate's last stable version
        // If `max_version` is `0.0.0` then all versions have been yanked
        if (!requestedVersion && maxVersion !== '0.0.0') {
            if (isUnstableVersion(maxVersion)) {
                crate
                    .get('versions')
                    .then(versions => {
                        const latestStableVersion = versions.find(version => {
                            if (!isUnstableVersion(version.get('num')) && !version.get('yanked')) {
                                return version;
                            }
                        });

                        if (latestStableVersion == null) {
                            // If no stable version exists, fallback to `maxVersion`
                            params.version_num = maxVersion;
                        } else {
                            params.version_num = latestStableVersion.get('num');
                        }
                    })
                    .then(fetchCrateDocumentation);
            } else {
                params.version_num = maxVersion;
                fetchCrateDocumentation();
            }
        } else {
            fetchCrateDocumentation();
        }

        controller.set('crate', crate);
        controller.set('requestedVersion', requestedVersion);
        controller.set('fetchingFollowing', true);

        if (this.get('session.currentUser')) {
            ajax(`/api/v1/crates/${crate.get('name')}/following`)
                .then(d => controller.set('following', d.following))
                .finally(() => controller.set('fetchingFollowing', false));
        }

        // Find version model
        let versions = await crate.get('versions');

        const version = versions.find(version => version.get('num') === params.version_num);
        if (params.version_num && !version) {
            this.flashMessages.queue(`Version '${params.version_num}' of crate '${crate.get('name')}' does not exist`);
        }

        const result = version || versions.find(version => version.get('num') === maxVersion) || versions.objectAt(0);

        if (result.get('readme_path')) {
            fetch(result.get('readme_path'))
                .then(async r => {
                    if (r.ok) {
                        crate.set('readme', await r.text());
                    } else {
                        crate.set('readme', null);
                    }
                })
                .catch(() => {
                    crate.set('readme', null);
                });
        }

        return result;
    },

    serialize(model) {
        let version_num = model.get('num');
        return { version_num };
    },
});
