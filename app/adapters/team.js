import ApplicationAdapter from './application';

export default class TeamAdapter extends ApplicationAdapter {
  queryRecord(store, type, query) {
    let url = this.urlForFindRecord(query.team_id, 'team');
    return this.ajax(url, 'GET');
  }
}
