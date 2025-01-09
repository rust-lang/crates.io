import { Factory } from 'miragejs';

export default Factory.extend({
  afterCreate(session) {
    if (!session.user) {
      throw new Error('Missing `user` relationship');
    }
  },
});
