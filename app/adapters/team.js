import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
  queryRecord(store, type, query) {
    let url = this.urlForFindRecord(query.team_id, 'team');
    return this.ajax(url, 'GET');
  },
});
