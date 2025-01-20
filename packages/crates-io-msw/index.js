import apiToken from './models/api-token.js';
import category from './models/category.js';
import crate from './models/crate.js';
import keyword from './models/keyword.js';
import mswSession from './models/msw-session.js';
import team from './models/team.js';
import user from './models/user.js';
import { factory } from './utils/factory.js';

export const handlers = [];

export const db = factory({
  apiToken,
  category,
  crate,
  keyword,
  mswSession,
  team,
  user,
});
