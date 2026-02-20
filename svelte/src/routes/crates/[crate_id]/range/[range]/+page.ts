import { resolve } from '$app/paths';
import { createClient } from '@crates-io/api-client';
import { error, redirect } from '@sveltejs/kit';
import maxSatisfying from 'semver/ranges/max-satisfying';

function cargoRangeToNpm(range: string): string {
  return range.replace(',', ' ');
}

export async function load({ fetch, params, parent }) {
  let { crate } = await parent();

  let crateName = params.crate_id;
  let range = params.range;

  let client = createClient({ fetch });

  let response;
  try {
    response = await client.GET('/api/v1/crates/{name}/versions', {
      params: { path: { name: crateName } },
    });
  } catch {
    error(500, { message: `${crate.name}: Failed to load version data`, tryAgain: true });
  }

  let status = response.response.status;
  if (response.error) {
    error(status, { message: `${crate.name}: Failed to load version data`, tryAgain: true });
  }

  let versions = response.data.versions;
  let allVersionNums = versions.map(v => v.num);
  let unyankedVersionNums = versions.filter(v => !v.yanked).map(v => v.num);

  let npmRange = cargoRangeToNpm(range);
  let versionNum = maxSatisfying(unyankedVersionNums, npmRange) ?? maxSatisfying(allVersionNums, npmRange);

  if (versionNum) {
    redirect(302, resolve('/crates/[crate_id]/[version_num]', { crate_id: crateName, version_num: versionNum }));
  }

  error(404, { message: `${crate.name}: No matching version found for ${range}` });
}
