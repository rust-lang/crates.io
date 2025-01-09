import { Factory } from 'miragejs';

export default Factory.extend({
  crateScopes: null,
  createdAt: '2017-11-19T17:59:22',
  endpointScopes: null,
  expiredAt: null,
  lastUsedAt: null,
  name: i => `API Token ${i + 1}`,
  token: () => generateToken(),

  afterCreate(model) {
    if (!model.user) {
      throw new Error('Missing `user` relationship on `api-token`');
    }
  },
});

function generateToken() {
  return Math.random().toString().slice(2);
}
