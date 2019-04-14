import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
    flashMessages: service(),

    redirect() {
        const crate = this.modelFor('crate');

        const repository = crate.get('repository');
        if (repository) {
            window.location = repository;
        } else {
            // Redirect to the crate's main page and show a flash error if
            // no repository is found
            const message = 'Crate does not supply a repository URL';
            this.flashMessages.queue(message);
            this.replaceWith('crate', crate);
        }
    },
});
