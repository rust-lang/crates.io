import { Factory, trait, faker } from 'ember-cli-mirage';

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
    license: () => faker.hacker.abbreviation(),
    max_version: () => faker.system.semver(),

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

    links() {
        return {
            'owner_user': `/api/v1/crates/${this.id}/owner_user`,
            'owner_team': `/api/v1/crates/${this.id}/owner_team`,
            'reverse_dependencies': `/api/v1/crates/${this.id}/reverse_dependencies`,
            'version_downloads': `/api/v1/crates/${this.id}/downloads`,
            'versions': `/api/v1/crates/${this.id}/versions`,
        };
    },

    withVersion: trait({
        afterCreate(crate, server) {
            server.create('version', { crate: crate.id });
        }
    }),
});
