import ApplicationAdapter from './application';

const BULK_REQUEST_GROUP_SIZE = 10;

export default class CrateAdapter extends ApplicationAdapter {
  coalesceFindRequests = true;

  async findHasMany(store, snapshot, url, relationship) {
    if (relationship.key === 'versions') {
      let { adapterOptions } = snapshot;
      let data;
      if (adapterOptions?.withReleaseTracks === true) {
        data = { include: 'release_tracks' };
      }
      return this.ajax(url, 'GET', { data }).then(resp => {
        let crate = store.peekRecord('crate', snapshot.id);
        if (resp.meta) {
          let payload = {
            crate: {
              id: snapshot.id,
              versions_meta: {
                ...crate.versions_meta,
                ...resp.meta,
              },
            },
          };
          store.pushPayload(payload);
        }
        return resp;
      });
    }

    return super.findHasMany(store, snapshot, url, relationship);
  }

  findRecord(store, type, id, snapshot) {
    let { include } = snapshot;
    // This ensures `crate.versions` are always fetched from another request.
    if (include === undefined) {
      snapshot.include = 'keywords,categories,badges,downloads';
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
