const DESCRIPTIONS: Record<string, string> = {
  'change-owners': 'Invite new crate owners, revoke invitations or remove owners',
  'crate-owner-invitations': 'List, accept and decline crate owner invitations you have received',
  'publish-new': 'Publish new crates',
  'publish-update': 'Publish new versions of existing crates',
  'trusted-publishing': 'Manage trusted publishing configurations',
  yank: 'Yank and unyank crate versions',
};

export function scopeDescription(scope: string): string {
  return DESCRIPTIONS[scope] ?? scope;
}
