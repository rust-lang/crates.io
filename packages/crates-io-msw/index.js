import category from './models/category.js';
import { factory } from './utils/factory.js';

export const handlers = [];

export const db = factory({
  category,
});
