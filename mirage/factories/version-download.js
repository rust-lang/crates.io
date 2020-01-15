import { Factory } from 'ember-cli-mirage';

export default Factory.extend({
  // version

  date: '2019-05-21',
  downloads: i => (((i * 42) % 13) + 4) * 2345,
});
