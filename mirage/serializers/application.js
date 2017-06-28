import { Serializer } from 'ember-cli-mirage';

export default Serializer.extend({
    keyForCollection(modelName) {
        if (modelName === 'version-download') {
            return 'version_downloads';
        }

        return Serializer.prototype.keyForCollection.apply(this, arguments);
    }
});
