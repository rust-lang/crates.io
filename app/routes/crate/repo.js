import Ember from 'ember';

export default Ember.Route.extend({
    redirect() {
        let crate = this.modelFor('crate');

        let repository = crate.get('repository');
        if (repository) {
            window.location = repository;
        } else {
            // Redirect to the crate's main page and show a flash error if
            // no repository is found
            let message = 'Crate does not supply a repository URL';
            this.controllerFor('application').set('nextFlashError', message);
            this.replaceWith('crate', crate);
        }
    },
});
