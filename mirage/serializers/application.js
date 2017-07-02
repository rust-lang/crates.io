import { ActiveModelSerializer } from 'ember-cli-mirage';

export default ActiveModelSerializer.extend({
    keyForCollection(modelName) {
        if (modelName === 'version-download') {
            return 'version_downloads';
        }

        return ActiveModelSerializer.prototype.keyForCollection.apply(this, arguments);
    }
});
