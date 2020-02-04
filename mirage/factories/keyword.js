import { Factory } from 'ember-cli-mirage';

export default Factory.extend({
  keyword: i => `keyword-${i + 1}`,

  id() {
    return this.keyword;
  },
});
