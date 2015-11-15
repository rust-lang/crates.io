import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
  model() {
    function addCrates(store, crates) {
        for (var i = 0; i < crates.length; i++) {
            const crate = crates[i];
            if (crate.versions == null) {
              // passing `null` will return an empty versions array
              delete crate.versions;
            }
            crates[i] = store.push(store.normalize('crate', crate));
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
