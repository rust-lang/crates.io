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
    ]
});
