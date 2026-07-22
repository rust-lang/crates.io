import type { ClientOptions } from 'openapi-fetch';
import type { paths } from './schema';

import createOpenAPIClient from 'openapi-fetch';

export { endpointScopeValues } from './schema';
export type { components, operations, paths } from './schema';

export function createClient(options?: ClientOptions) {
  return createOpenAPIClient<paths>(options);
}
