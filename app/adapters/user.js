import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
  queryRecord(store, type, query) {
    let url = this.urlForFindRecord(query.user_id, 'user');
    return this.ajax(url, 'GET');
  },
});
