import createOpenAPIClient, { type ClientOptions } from 'openapi-fetch';

import type { paths } from './schema';

export type { components, operations, paths } from './schema';

export function createClient(options?: ClientOptions) {
  return createOpenAPIClient<paths>(options);
}
