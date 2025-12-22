import createOpenAPIClient, { type ClientOptions } from 'openapi-fetch';

import type { components, operations, paths } from './schema';

export type { components, operations, paths };

export function createClient(options?: ClientOptions) {
  return createOpenAPIClient<paths>(options);
}
