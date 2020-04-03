import { EmbeddedRecordsMixin } from '@ember-data/serializer/rest';

import ApplicationSerializer from './application';

export default ApplicationSerializer.extend(EmbeddedRecordsMixin, {
  attrs: {
    published_by: { embedded: 'always' },
  },
});
