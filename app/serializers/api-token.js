import ApplicationSerializer from './application';

export default ApplicationSerializer.extend({
  payloadKeyFromModelName() {
    return 'api_token';
  },
});
