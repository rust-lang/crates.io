import Ember from 'ember';
import ajax from 'ember-fetch/ajax';

export default Ember.Route.extend({
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

        return ajax('/summary').then((data) => {
            addCrates(this.store, data.new_crates);
            addCrates(this.store, data.most_downloaded);
            addCrates(this.store, data.just_updated);

            return data;
        });
    }
});
