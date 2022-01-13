import { Factory } from 'miragejs';

export default Factory.extend({
  keyword: i => `keyword-${i + 1}`,

  id() {
    return this.keyword;
  },
});
