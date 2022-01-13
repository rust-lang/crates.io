import { Factory } from 'miragejs';

export default Factory.extend({
  emailNotifications: true,

  afterCreate(model) {
    if (!model.crate) {
      throw new Error('Missing `crate` relationship on `crate-ownership`');
    }
    if (model.team && model.user) {
      throw new Error('`team` and `user` on a `crate-ownership` are mutually exclusive');
    }
  },
});
