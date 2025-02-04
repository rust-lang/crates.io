import { describe, test } from 'vitest';

import { dasherize, underscore } from './strings.js';

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

describe('underscore', () => {
  function assert(input, expected) {
    test(input, ({ expect }) => {
      expect(underscore(input)).toBe(expected);
    });
  }

  assert('my favorite items', 'my_favorite_items');
  assert('css-class-name', 'css_class_name');
  assert('action_name', 'action_name');
  assert('innerHTML', 'inner_html');
  assert('PrivateDocs/OwnerInvoice', 'private_docs/owner_invoice');
  assert('privateDocs/ownerInvoice', 'private_docs/owner_invoice');
  assert('private-docs/owner-invoice', 'private_docs/owner_invoice');
});
