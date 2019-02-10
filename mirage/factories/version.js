import { Factory, faker } from 'ember-cli-mirage';

export default Factory.extend({
    id: i => i,

    // crate: '...',

    num: () => faker.system.semver(),

    created_at: () => faker.date.past(),
    updated_at() {
        return faker.date.between(this.created_at, new Date());
    },

    yanked: false,
    license: () => faker.hacker.abbreviation(),

    dl_path() {
        return `/api/v1/crates/${this.crate}/${this.num}/download`;
    },

    downloads: () => faker.random.number({ max: 10000 }),
    features: () => {},
    _authors: () => [],

    afterCreate(version, server) {
        let crate = server.schema.crates.find(version.crate);
        crate.update({ versions: crate.versions.concat(parseInt(version.id, 10)) });
    },
    crate_size: () => faker.random.number({ max: 10000000 }),
});
