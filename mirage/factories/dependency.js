import { Factory } from 'ember-cli-mirage';

const REQS = ['^0.1.0', '^2.1.3', '0.3.7', '~5.2.12'];

export default Factory.extend({
  default_features: i => i % 4 === 3,
  features: () => [],
  kind: i => (i % 3 === 0 ? 'dev' : 'normal'),
  optional: i => i % 4 !== 3,
  req: i => REQS[i % REQS.length],
  target: null,

  afterCreate(self) {
    if (!self.crateId) {
      throw new Error(`Missing \`crate\` relationship on \`dependency:${self.id}\``);
    }
    if (!self.versionId) {
      throw new Error(`Missing \`version\` relationship on \`dependency:${self.id}\``);
    }
  },
});
