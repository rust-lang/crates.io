import { Factory } from 'ember-cli-mirage';

export default Factory.extend({
  date: '2019-05-21',
  downloads: i => (((i * 42) % 13) + 4) * 2345,

  afterCreate(self) {
    if (!self.versionId) {
      throw new Error(`Missing \`version\` relationship on \`version-download:${self.date}\``);
    }
  },
});
