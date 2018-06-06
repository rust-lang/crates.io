import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
    flashMessages: service(),

    model({ keyword_id }) {
        return this.store.find('keyword', keyword_id).catch(e => {
            if (e.errors.some(e => e.detail === 'Not Found')) {
                this.flashMessages.queue(`Keyword '${keyword_id}' does not exist`);
                return this.replaceWith('index');
            }
        });
    },
});
