import { factory as mswFactory } from '@mswjs/data';

export function factory(models) {
  // Create a new MSW database instance with the given models.
  return mswFactory(models);
}
