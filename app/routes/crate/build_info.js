import Ember from 'ember';

export default Ember.Route.extend({
    model(params) {
        const requestedVersion = params.version_num;
        const crate = this.modelFor('crate');

        // Find version model
        return crate.get('versions')
            .then(versions => {
                const version = versions.find(version => version.get('num') === requestedVersion);
                if (!version) {
                    this.controllerFor('application').set('nextFlashError',
                        `Version '${requestedVersion}' of crate '${crate.get('name')}' does not exist`);
                }
                return version;
            });
    },

    serialize(model) {
        let version_num = model ? model.get('num') : '';
        return { version_num };
    },
});
