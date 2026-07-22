import type { CrateOwnerInvitation } from '../models/index.js';

import { serializeModel } from '../utils/serializers.js';

export function serializeInvite(invite: CrateOwnerInvitation) {
  let serialized = serializeModel(invite);

  serialized.crate_id = serialized.crate.id;
  serialized.crate_name = serialized.crate.name;
  serialized.invitee_id = serialized.invitee.id;
  serialized.inviter_id = serialized.inviter.id;

  delete serialized.id;
  delete serialized.token;
  delete serialized.crate;
  delete serialized.invitee;
  delete serialized.inviter;

  return serialized;
}

export function serializeLegacyInvite(invite: CrateOwnerInvitation) {
  return {
    ...serializeInvite(invite),
    invited_by_username: invite.inviter.login,
  };
}
