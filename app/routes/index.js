import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
  model() {
    function addCrates(store, crates) {
        for (var i = 0; i < crates.length; i++) {
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
