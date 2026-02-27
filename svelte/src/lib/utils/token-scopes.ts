const DESCRIPTIONS: Record<string, string> = {
  'change-owners': 'Invite new crate owners or remove existing ones',
  'publish-new': 'Publish new crates',
  'publish-update': 'Publish new versions of existing crates',
  'trusted-publishing': 'Manage trusted publishing configurations',
  yank: 'Yank and unyank crate versions',
};

export function scopeDescription(scope: string): string {
  return DESCRIPTIONS[scope] ?? scope;
}
