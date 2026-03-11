export async function load({ parent }) {
  let { ownersPromise } = await parent();
  let owners = await ownersPromise;
  return { owners };
}
