import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
    flashMessages: service(),

    model(params) {
        return this.store.find('category', params.category_id).catch(e => {
            if (e.errors.some(e => e.detail === 'Not Found')) {
                this.flashMessages.queue(`Category '${params.category_id}' does not exist`);
                return this.replaceWith('index');
            }
        });
    },
});
