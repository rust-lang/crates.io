import { Factory } from 'ember-cli-mirage';
import { dasherize } from '@ember/string';

export default Factory.extend({
  category: i => `Category ${i}`,

  slug() {
    return dasherize(this.category);
  },

  id() {
    return this.slug;
  },

  description() {
    return `This is the description for the category called "${this.category}"`;
  },

  created_at: '2010-06-16T21:30:45Z',
});
