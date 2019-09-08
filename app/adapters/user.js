import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
  stats(id) {
    return this.ajax(this.urlForStatsAction(id), 'GET');
  },

  urlForStatsAction(id) {
    return `${this.buildURL('user', id)}/stats`;
  },

  queryRecord(store, type, query) {
    let url = this.urlForFindRecord(query.user_id, 'user');
    return this.ajax(url, 'GET');
  },
});
