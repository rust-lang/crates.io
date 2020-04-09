import Component from '@ember/component';
import { inject as service } from '@ember/service';

export default Component.extend({
  googleCharts: service(),

  tagName: '',

  didInsertElement() {
    this.googleCharts.load();
  },
});
