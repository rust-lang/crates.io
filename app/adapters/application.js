import RESTAdapter from '@ember-data/adapter/rest';

export default class ApplicationAdapter extends RESTAdapter {
  namespace = 'api/v1';

  isInvalid() {
    // HTTP 422 errors are causing all sorts of issues within Ember Data,
    // so we disable their special case handling here, since we don't need/want it.
    return false;
  }

  handleResponse(status, headers, payload, requestData) {
    if (typeof payload === 'string') {
      try {
        payload = JSON.parse(payload);
      } catch {
        // if the payload can't be parsed as JSON then let's continue
        // with the string payload
      }
    }

    return super.handleResponse(status, headers, payload, requestData);
  }
}
