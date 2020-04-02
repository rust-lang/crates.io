import { Factory } from 'ember-cli-mirage';

export default Factory.extend({
  afterCreate(session) {
    if (!session.user) {
      throw new Error('Missing `user` relationship');
    }
  },
});
