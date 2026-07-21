import type { operations } from '@crates-io/api-client';
import type { SuccessBody } from './api-types.js';

import { expectTypeOf, test } from 'vitest';

test('extracts successful JSON response bodies', () => {
  type FindCrateBody = operations['find_crate']['responses'][200]['content']['application/json'];
  type ListCategoriesBody = operations['list_categories']['responses'][200]['content']['application/json'];

  expectTypeOf<SuccessBody<'find_crate'>>().toEqualTypeOf<FindCrateBody>();
  expectTypeOf<SuccessBody<'find_crate', 200>>().toEqualTypeOf<FindCrateBody>();
  expectTypeOf<SuccessBody<'list_categories'>>().toEqualTypeOf<ListCategoriesBody>();
});

test('excludes successful responses without JSON content', () => {
  expectTypeOf<SuccessBody<'delete_crate'>>().toBeNever();
});

test('rejects unknown operations and response statuses', () => {
  // @ts-expect-error `missing_operation` is not part of the generated API schema
  expectTypeOf<SuccessBody<'missing_operation'>>();

  // @ts-expect-error `find_crate` only documents a `200` response
  expectTypeOf<SuccessBody<'find_crate', 201>>();
});
