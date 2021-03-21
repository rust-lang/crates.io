import ApplicationAdapter from './application';

const BULK_REQUEST_GROUP_SIZE = 10;

export default class CrateAdapter extends ApplicationAdapter {
  coalesceFindRequests = true;

  groupRecordsForFindMany(store, snapshots) {
    let result = [];
    for (let i = 0; i < snapshots.length; i += BULK_REQUEST_GROUP_SIZE) {
      result.push(snapshots.slice(i, i + BULK_REQUEST_GROUP_SIZE));
    }
    return result;
  }
}
