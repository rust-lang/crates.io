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

    dl_path() {
        return `/api/v1/crates/${this.crate}/${this.num}/download`;
    },

    downloads: () => faker.random.number({ max: 10000 }),
    features: () => {},
    _authors: () => [],

    links() {
        return {
            'authors': `/api/v1/crates/${this.crate}/${this.num}/authors`,
            'dependencies': `/api/v1/crates/${this.crate}/${this.num}/dependencies`,
            'version_downloads': `/api/v1/crates/${this.crate}/${this.num}/downloads`,
        };
    },

    afterCreate(version, server) {
        let crate = server.schema.crates.find(version.crate);
        crate.update({ versions: crate.versions.concat(parseInt(version.id, 10)) });
    }
});
