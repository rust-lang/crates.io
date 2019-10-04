import { ActiveModelSerializer } from 'ember-cli-mirage';

export default ActiveModelSerializer.extend({
  getHashForResource(resource) {
    let isModel = this.isModel(resource);
    let hash = ActiveModelSerializer.prototype.getHashForResource.apply(this, arguments);

    if (isModel) {
      let links = this.links(resource);
      if (links) {
        hash[0].links = links;
      }
    } else {
      for (let i = 0; i < hash[0].length && i < resource.models.length; i++) {
        let links = this.links(resource.models[i]);
        if (links) {
          hash[0][i].links = links;
        }
      }
    }

    return hash;
  },

  links() {},
});
