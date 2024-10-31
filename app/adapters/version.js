import ApplicationAdapter from './application';

export default class VersionAdapter extends ApplicationAdapter {
  urlForUpdateRecord(id, modelName, snapshot) {
    let crateName = snapshot.record.crate.id;
    let num = snapshot.record.num;
    return `/${this.namespace}/crates/${crateName}/${num}`;
  }
}
