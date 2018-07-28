import BaseSerializer from './application';

export default BaseSerializer.extend({
    attrs: [
        'badges',
        'categories',
        'created_at',
        'description',
        'documentation',
        'downloads',
        'recent_downloads',
        'homepage',
        'id',
        'keywords',
        'links',
        'max_version',
        'name',
        'repository',
        'updated_at',
        'versions',
    ],

    links(crate) {
        return {
            owner_user: `/api/v1/crates/${crate.id}/owner_user`,
            owner_team: `/api/v1/crates/${crate.id}/owner_team`,
            reverse_dependencies: `/api/v1/crates/${crate.id}/reverse_dependencies`,
            version_downloads: `/api/v1/crates/${crate.id}/downloads`,
            versions: `/api/v1/crates/${crate.id}/versions`,
        };
    },
});
