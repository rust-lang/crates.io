import { Factory } from 'ember-cli-mirage';
import faker from 'faker';

export default Factory.extend({
  id(i) {
    return `crate-${i}`;
  },

  name() {
    return this.id;
  },

  description: () => faker.lorem.sentence(),
  downloads: () => faker.random.number({ max: 10000 }),
  documentation: () => faker.internet.url(),
  homepage: () => faker.internet.url(),
  repository: () => faker.internet.url(),
  max_version: () => faker.system.semver(),
  newest_version: () => faker.system.semver(),

  created_at: () => faker.date.past(),
  updated_at() {
    return faker.date.between(this.created_at, new Date());
  },

  badges: () => [],
  categories: () => [],
  keywords: () => [],
  versions: () => [],
  _extra_downloads: () => [],
  _owner_teams: () => [],
  _owner_users: () => [],
});
