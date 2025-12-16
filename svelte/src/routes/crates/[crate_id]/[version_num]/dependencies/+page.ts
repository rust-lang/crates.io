export function load({ params }) {
  return { crate_id: params.crate_id, version_num: params.version_num };
}
