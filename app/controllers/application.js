import Controller from '@ember/controller';
import { inject as service } from '@ember/service';

export default Controller.extend({
  design: service(),
  flashMessages: service(),
  progress: service(),
});
