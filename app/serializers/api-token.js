import ApplicationSerializer from './application';

export default class ApiTokenSerializer extends ApplicationSerializer {
  payloadKeyFromModelName() {
    return 'api_token';
  }
}
