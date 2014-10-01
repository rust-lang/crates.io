import Ember from 'ember';
import Version from 'cargo/models/version';
import Crate from 'cargo/models/crate';

export default Ember.Route.extend({
    setupController: function(controller, data) {
        if (data instanceof Crate) {
            data = {crate: data, version: null};
        } else if (data instanceof Version) {
            data = {crate: data.get('crate'), version: data.get('num')};
        }
        var self = this;
        this._super(controller, data.crate);
        controller.set('showAllVersions', false);
        controller.set('fetchingVersions', true);
        controller.set('fetchingDownloads', true);

        // Try to find the requested version in the versions we fetch
        var max = data.crate.get('max_version');
        data.crate.get('versions').then(function(array) {
            controller.set('fetchingVersions', false);
            var hit = array.any(function(version) {
                return data.version === version.get('num') ||
                           (data.version == null && version.get('num') === max);
            });
            if (!hit) {
                var msg = "Version `" + data.version + "` does not exist";
                self.controllerFor('application').set('flashError', msg);
                data.version = null;
            }
            controller.set('requestedVersion', data.version);
            array.forEach(function(version) {
                if (data.version === version.get('num') ||
                    (data.version == null && version.get('num') === max)) {
                    controller.set('currentVersion', version);
                }
            });
        }).then(function() {
            return controller.get('currentVersion').get('version_downloads');
        }).then(function(downloads) {
            controller.set('fetchingDownloads', false);
            controller.send('renderChart', downloads);
        });
    },
});
