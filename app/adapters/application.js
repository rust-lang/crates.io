import RESTAdapter from '@ember-data/adapter/rest';

export default class ApplicationAdapter extends RESTAdapter {
  namespace = 'api/v1';

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
