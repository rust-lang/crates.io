import ApplicationAdapter from './application';

const BULK_REQUEST_GROUP_SIZE = 10;

export default class CrateAdapter extends ApplicationAdapter {
  coalesceFindRequests = true;

  findRecord(store, type, id, snapshot) {
    return super.findRecord(store, type, id, setDefaultInclude(snapshot));
  }

  queryRecord(store, type, query, adapterOptions) {
    return super.queryRecord(store, type, setDefaultInclude(query), adapterOptions);
  }

  /** Removes the `name` query parameter and turns it into a path parameter instead */
  urlForQueryRecord(query) {
    let baseUrl = super.urlForQueryRecord(...arguments);
    if (!query.name) {
      return baseUrl;
    }

    let crateName = query.name;
    delete query.name;
    return `${baseUrl}/${crateName}`;
  }

  /** Adds a `message` query parameter to the URL, if set in the `adapterOptions`. */
  urlForDeleteRecord(id, modelName, snapshot) {
    let url = super.urlForDeleteRecord(...arguments);

    let message = snapshot.adapterOptions.message;
    if (message) {
      url += `?message=${encodeURIComponent(message)}`;
    }

    return url;
  }

  groupRecordsForFindMany(store, snapshots) {
    let result = [];
    for (let i = 0; i < snapshots.length; i += BULK_REQUEST_GROUP_SIZE) {
      result.push(snapshots.slice(i, i + BULK_REQUEST_GROUP_SIZE));
    }
    return result;
  }
}

function setDefaultInclude(query) {
  if (query.include === undefined) {
    // This ensures `crate.versions` are always fetched from another request.
    query.include = 'keywords,categories,downloads';
  }

  return query;
}
