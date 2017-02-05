import Ember from 'ember';

export default Ember.Route.extend({
    redirect() {
        var crate = this.modelFor('crate');

        var repository = crate.get('repository');
        if (repository) {
            window.location = repository;
        } else {
            // Redirect to the crate's main page and show a flash error if
            // no repository is found
            var message = 'Crate does not supply a repository URL';
            this.controllerFor('application').set('nextFlashError', message);
            this.replaceWith('crate', crate);
        }
    },
});
