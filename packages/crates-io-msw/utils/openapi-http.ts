import type { paths } from '@crates-io/api-client';

import { createOpenApiHttp } from 'openapi-msw';

/** OpenAPI-aware MSW handler factories for the crates.io API. */
export const http = createOpenApiHttp<paths>();
