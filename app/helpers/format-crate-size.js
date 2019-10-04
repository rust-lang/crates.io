import { helper } from '@ember/component/helper';

export function formatCrateSize(sizeInBytes) {
  if (sizeInBytes < 100000) {
    return +(sizeInBytes / 1000).toFixed(2) + ' kB';
  } else {
    return +(sizeInBytes / 1000000).toFixed(2) + ' MB';
  }
}

export default helper(formatCrateSize);
