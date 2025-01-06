import ApplicationAdapter from './application';

const BULK_REQUEST_GROUP_SIZE = 10;

export default class CrateAdapter extends ApplicationAdapter {
  coalesceFindRequests = true;

  findRecord(store, type, id, snapshot) {
    let { include } = snapshot;
    // This ensures `crate.versions` are always fetched from another request.
    if (include === undefined) {
      snapshot.include = 'keywords,categories,downloads';
    }
    return super.findRecord(store, type, id, snapshot);
  }

  groupRecordsForFindMany(store, snapshots) {
    let result = [];
    for (let i = 0; i < snapshots.length; i += BULK_REQUEST_GROUP_SIZE) {
      result.push(snapshots.slice(i, i + BULK_REQUEST_GROUP_SIZE));
    }
    return result;
  }
}
