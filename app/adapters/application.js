import RESTAdapter from '@ember-data/adapter/rest';
import { computed } from '@ember/object';
import { inject as service } from '@ember/service';

export default RESTAdapter.extend({
  fastboot: service(),
  fetcher: service(),

  namespace: 'api/v1',

  ajax(url, type, options) {
    if (type === 'GET') {
      let cache = this.fetcher.get(url, options);
      if (cache) {
        return cache;
      }
    }

    return this._super(url, type, options).then(resp => {
      this.fetcher.put(url, options, resp);
      return resp;
    });
  },

  headers: computed('fastboot.{isFastBoot,request.headers}', function () {
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
