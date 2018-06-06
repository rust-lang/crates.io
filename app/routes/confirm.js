import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import ajax from 'ember-fetch/ajax';

export default Route.extend({
    flashMessages: service(),
    session: service(),

    async model(params) {
        try {
            await ajax(`/api/v1/confirm/${params.email_token}`, { method: 'PUT', body: '{}' });

            /*  We need this block to reload the user model from the database,
                without which if we haven't submitted another GET /me after
                clicking the link and before checking their account info page,
                the user will still see that their email has not yet been
                validated and could potentially be confused, resend the email,
                and set up a situation where their email has been verified but
                they have an unverified token sitting in the DB.

                Suggestions of a more ideomatic way to fix/test this are welcome!
            */
            if (this.get('session.isLoggedIn')) {
                ajax('/api/v1/me').then(response => {
                    this.session.set('currentUser', this.store.push(this.store.normalize('user', response.user)));
                });
            }
        } catch (error) {
            if (error.payload) {
                this.flashMessages.queue(`Error in email confirmation: ${error.payload.errors[0].detail}`);
                return this.replaceWith('index');
            } else {
                this.flashMessages.queue(`Unknown error in email confirmation`);
                return this.replaceWith('index');
            }
        }
    },
});
