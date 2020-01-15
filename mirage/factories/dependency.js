import { Factory } from 'ember-cli-mirage';

const REQS = ['^0.1.0', '^2.1.3', '0.3.7', '~5.2.12'];

export default Factory.extend({
  // crate_id,
  // version_id,

  default_features: i => i % 4 === 3,
  features: () => [],
  kind: i => (i % 3 === 0 ? 'dev' : 'normal'),
  optional: i => i % 4 !== 3,
  req: i => REQS[i % REQS.length],
  target: null,
});
