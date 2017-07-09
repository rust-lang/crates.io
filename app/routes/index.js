import Ember from 'ember';
import FastBootUtils from 'cargo/mixins/fastboot-utils';

const { inject: { service } } = Ember;

export default Ember.Route.extend(FastBootUtils, {

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

        let summaryURL = `${this.get('appURL')}/summary`;

        return this.get('ajax').request(summaryURL).then((data) => {
            addCrates(this.store, data.new_crates);
            addCrates(this.store, data.most_downloaded);
            addCrates(this.store, data.just_updated);

            return data;
        });
    }
});
