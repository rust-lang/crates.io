import ApplicationAdapter from './application';

export default class UserAdapter extends ApplicationAdapter {
  queryRecord(store, type, query) {
    let url = this.urlForFindRecord(query.user_id, 'user');
    return this.ajax(url, 'GET');
  }
}
