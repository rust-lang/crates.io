export function compareDates(a: string, b: string) {
  let aDate = new Date(a);
  let bDate = new Date(b);
  return aDate < bDate ? -1 : aDate > bDate ? 1 : 0;
}
