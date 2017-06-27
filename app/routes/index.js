import Ember from 'ember';
import ajax from 'ember-fetch/ajax';

const { inject: { service } } = Ember;

export default Ember.Route.extend({

    fastboot: service(),

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

        let summaryURL = `/summary`;
        if (this.get('fastboot.isFastBoot')) {
            let protocol = this.get('fastboot.request.protocol');
            let host = this.get('fastboot.request.host');
            summaryURL = `${protocol}://${host}/summary`;
        }

        return ajax(summaryURL).then((data) => {
            addCrates(this.store, data.new_crates);
            addCrates(this.store, data.most_downloaded);
            addCrates(this.store, data.just_updated);

            return data;
        });
    }
});
