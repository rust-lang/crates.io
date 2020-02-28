import RESTAdapter from '@ember-data/adapter/rest';
import { inject as service } from '@ember/service';
import { computed } from '@ember/object';

export default RESTAdapter.extend({
  fastboot: service(),

  namespace: 'api/v1',

  headers: computed('fastboot.{isFastBoot,request.headers}', function() {
    if (this.fastboot.isFastBoot) {
      return { 'User-Agent': this.fastboot.request.headers.get('User-Agent') };
    }

    return {};
  }),

  handleResponse(status, headers, payload, requestData) {
    if (typeof payload === 'string') {
      try {
        payload = JSON.parse(payload);
      } catch (ignored) {
        // if the payload can't be parsed as JSON then let's continue
        // with the string payload
      }
    }

    return this._super(status, headers, payload, requestData);
  },
});
