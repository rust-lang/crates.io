import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';

import ajax from 'ember-fetch/ajax';
import { task } from 'ember-concurrency';

export default Controller.extend({
    model: readOnly('dataTask.lastSuccessful.value'),

    hasData: computed('dataTask.{lastSuccessful,isRunning}', function() {
        return this.get('dataTask.lastSuccessful') || !this.get('dataTask.isRunning');
    }),

    dataTask: task(function*() {
        let data = yield ajax('/api/v1/summary');

        addCrates(this.store, data.new_crates);
        addCrates(this.store, data.most_downloaded);
        addCrates(this.store, data.just_updated);
        addCrates(this.store, data.most_recently_downloaded);

        return data;
    }).drop(),
});

function addCrates(store, crates) {
    for (let i = 0; i < crates.length; i++) {
        crates[i] = store.push(store.normalize('crate', crates[i]));
    }
}
