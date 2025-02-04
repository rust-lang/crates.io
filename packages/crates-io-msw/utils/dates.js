export function compareDates(a, b) {
  let aDate = new Date(a);
  let bDate = new Date(b);
  return aDate < bDate ? -1 : aDate > bDate ? 1 : 0;
}
