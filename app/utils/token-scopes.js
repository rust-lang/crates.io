import { htmlSafe } from '@ember/template';

const DESCRIPTIONS = {
  'change-owners': 'Invite new crate owners or remove existing ones',
  'publish-new': 'Publish new crates',
  'publish-update': 'Publish new versions of existing crates',
  yank: 'Yank and unyank crate versions',
};

export function scopeDescription(scope) {
  return DESCRIPTIONS[scope];
}

export function patternDescription(pattern) {
  if (pattern === '*') {
    return 'Matches all crates on crates.io';
  } else if (pattern.endsWith('*')) {
    return htmlSafe(`Matches all crates starting with <strong>${pattern.slice(0, -1)}</strong>`);
  } else {
    return htmlSafe(`Matches only the <strong>${pattern}</strong> crate`);
  }
}
