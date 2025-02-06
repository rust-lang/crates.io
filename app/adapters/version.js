import ApplicationAdapter from './application';

export default class VersionAdapter extends ApplicationAdapter {
  urlForUpdateRecord(id, modelName, snapshot) {
    let crateName = snapshot.record.crate.id;
    let num = snapshot.record.num;
    return `/${this.namespace}/crates/${crateName}/${num}`;
  }

  urlForQueryRecord(query) {
    let { name, num } = query ?? {};
    let baseUrl = this.buildURL('crate', name);
    let url = `${baseUrl}/${num}`;
    // The following used to remove them from URL's query string.
    delete query.name;
    delete query.num;
    return url;
  }
}
