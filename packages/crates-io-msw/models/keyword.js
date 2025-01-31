import { primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';

export default {
  id: primaryKey(String),

  keyword: String,

  preCreate(attrs, counter) {
    applyDefault(attrs, 'keyword', () => `keyword-${counter}`);
    applyDefault(attrs, 'id', () => attrs.keyword);
  },
};
