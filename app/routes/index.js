import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({

    ajax: service(),

    headTags: [{
        type: 'meta',
        attrs: {
            name: 'description',
            content: 'cargo is the package manager and crate host for rust'
        }
    }],

    model() {
        function addCrates(store, crates) {
            for (let i = 0; i < crates.length; i++) {
                crates[i] = store.push(store.normalize('crate', crates[i]));
            }
        }

        return this.get('ajax').request('/api/v1/summary').then((data) => {
            addCrates(this.store, data.new_crates);
            addCrates(this.store, data.most_downloaded);
            addCrates(this.store, data.just_updated);

            return data;
        });
    }
});
