import BaseSerializer from './application';

export default BaseSerializer.extend({
    attrs: [
        'badges',
        'categories',
        'created_at',
        'description',
        'documentation',
        'downloads',
        'homepage',
        'id',
        'keywords',
        'license',
        'links',
        'max_version',
        'name',
        'repository',
        'updated_at',
        'versions',
    ]
});
