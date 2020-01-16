import BaseSerializer from './application';

export default BaseSerializer.extend({
  getHashForResource() {
    let [hash, addToIncludes] = BaseSerializer.prototype.getHashForResource.apply(this, arguments);

    if (Array.isArray(hash)) {
      for (let resource of hash) {
        this._adjust(resource);
      }
    } else {
      this._adjust(hash);
    }

    return [hash, addToIncludes];
  },

  _adjust(hash) {
    let allCrates = this.schema.crates.all();
    let associatedCrates = allCrates.filter(it => it.keywordIds.includes(hash.id));

    hash.crates_cnt = associatedCrates.length;
  },
});
