import { Factory } from 'ember-cli-mirage';

const LICENSES = ['MIT/Apache-2.0', 'MIT', 'Apache-2.0'];

export default Factory.extend({
  num: i => `1.0.${i}`,

  created_at: '2010-06-16T21:30:45Z',
  updated_at: '2017-02-24T12:34:56Z',

  yanked: false,
  license: i => LICENSES[i % LICENSES.length],

  downloads: i => (((i + 13) * 42) % 13) * 1234,

  features: () => {},
  _authors: () => [],

  crate_size: i => (((i + 13) * 42) % 13) * 54321,

  afterCreate(version) {
    if (!version.crateId) {
      throw new Error(`Missing \`crate\` relationship on \`version:${version.num}\``);
    }
  },
});
