import { Collection } from '@msw/data';
import * as v from 'valibot';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

const schema = v.object({
  id: v.number(),

  createdAt: v.string(),
  expiresAt: v.string(),
  token: v.string(),

  crate: v.any(),
  invitee: v.any(),
  inviter: v.any(),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'id', () => counter);
  applyDefault(attrs, 'createdAt', () => '2016-12-24T12:34:56Z');
  applyDefault(attrs, 'expiresAt', () => '2017-01-24T12:34:56Z');
  applyDefault(attrs, 'token', () => `secret-token-${attrs.id}`);

  if (!attrs.crate) {
    throw new Error(`Missing \`crate\` relationship on \`crate-owner-invitation\``);
  }
  if (!attrs.invitee) {
    throw new Error(`Missing \`invitee\` relationship on \`crate-owner-invitation\``);
  }
  if (!attrs.inviter) {
    throw new Error(`Missing \`inviter\` relationship on \`crate-owner-invitation\``);
  }
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
