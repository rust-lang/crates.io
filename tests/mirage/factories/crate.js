import { Factory } from 'miragejs';

export default Factory.extend({
  name: i => `crate-${i}`,

  description() {
    return `This is the description for the crate called "${this.name}"`;
  },

  downloads: i => (((i + 13) * 42) % 13) * 12_345,

  documentation: null,
  homepage: null,
  repository: null,

  created_at: '2010-06-16T21:30:45Z',
  updated_at: '2017-02-24T12:34:56Z',

  badges: () => [],
  _extra_downloads: () => [],
});
