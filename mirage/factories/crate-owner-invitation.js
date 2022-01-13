import { Factory } from 'miragejs';

export default Factory.extend({
  createdAt: '2016-12-24T12:34:56Z',
  token: i => `secret-token-${i}`,

  afterCreate(invite) {
    if (!invite.crateId) {
      throw new Error(`Missing \`crate\` relationship on \`crate-owner-invitation:${invite.id}\``);
    }
    if (!invite.inviteeId) {
      throw new Error(`Missing \`invitee\` relationship on \`crate-owner-invitation:${invite.id}\``);
    }
    if (!invite.inviterId) {
      throw new Error(`Missing \`inviter\` relationship on \`crate-owner-invitation:${invite.id}\``);
    }
  },
});
