import BaseSerializer from './application';

export default BaseSerializer.extend({
    attrs: [
        'crate',
        'created_at',
        'dl_path',
        'downloads',
        'features',
        'id',
        'links',
        'num',
        'updated_at',
        'yanked',
        'license',
        'crate_size',
    ],

    links(version) {
        return {
            authors: `/api/v1/crates/${version.crate}/${version.num}/authors`,
            dependencies: `/api/v1/crates/${version.crate}/${version.num}/dependencies`,
            version_downloads: `/api/v1/crates/${version.crate}/${version.num}/downloads`,
        };
    },
});
