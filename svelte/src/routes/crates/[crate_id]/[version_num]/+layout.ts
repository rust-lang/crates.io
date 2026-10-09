import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch, params }) {
  let client = createClient({ fetch });

  let crateName = params.crate_id;
  let versionNum = params.version_num;

  let { version } = await loadApi(
    client.GET('/api/v1/crates/{name}/{version}', { params: { path: { name: crateName, version: versionNum } } }),
    {
      message: `${crateName}: Failed to load version data`,
      notFound: `${crateName}: Version ${versionNum} not found`,
    },
  );

  return { requestedVersion: versionNum, version };
}
