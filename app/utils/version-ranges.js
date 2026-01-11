// Extracts version ranges from a RustSec advisory
//
// OSV interleaves `introduced` and `fixed` events to represent ranges, like so:
//
// ```
// "events": [
//   { introduced: "0.0.0-0" },
//   { fixed: "0.7.46" },
//   { introduced: "0.8.0" },
//   { fixed: "0.8.13" }
// ]
// ```
export function versionRanges(advisory) {
  if (!advisory.affected || advisory.affected.length === 0) {
    return null;
  }

  let ranges = [];
  for (let affected of advisory.affected) {
    if (affected.ranges.length === 0) {
      continue;
    }

    for (let range of affected.ranges) {
      if (range.type !== 'SEMVER' || range.events.length === 0) {
        continue;
      }

      let currentIntroduced = null;
      for (let event of range.events) {
        if (event.introduced !== undefined) {
          currentIntroduced = event.introduced;
        } else if (event.fixed !== undefined) {
          if (currentIntroduced === null || currentIntroduced === '0.0.0-0') {
            ranges.push(`<${event.fixed}`);
          } else {
            let start = currentIntroduced === '0.0.0-0' ? '0' : currentIntroduced;
            ranges.push(`>=${start}, <${event.fixed}`);
            currentIntroduced = null;
          }
        }
      }
    }
  }

  return ranges.length === 0 ? null : ranges.join('; ');
}
