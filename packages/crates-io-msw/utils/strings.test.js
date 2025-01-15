import { describe, test } from 'vitest';

import { dasherize } from './strings.js';

describe('dasherize', () => {
  function assert(input, expected) {
    test(input, ({ expect }) => {
      expect(dasherize(input)).toBe(expected);
    });
  }

  assert('my favorite items', 'my-favorite-items');
  assert('css-class-name', 'css-class-name');
  assert('action_name', 'action-name');
  assert('innerHTML', 'inner-html');
  assert('toString', 'to-string');
  assert('PrivateDocs/OwnerInvoice', 'private-docs/owner-invoice');
  assert('privateDocs/ownerInvoice', 'private-docs/owner-invoice');
  assert('private_docs/owner_invoice', 'private-docs/owner-invoice');
});
