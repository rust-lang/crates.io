export function seededRandom(seed) {
  return mulberry32(seed)();
}

function mulberry32(a) {
  return function () {
    let t = (a += 0x6d_2b_79_f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4_294_967_296;
  };
}
