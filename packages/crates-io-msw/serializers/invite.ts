import type { components } from '@crates-io/api-client';
import type { CrateOwnerInvitation } from '../models/index.js';

type ApiCrateOwnerInvitation = components['schemas']['CrateOwnerInvitation'];

export function serializeInvite(invite: CrateOwnerInvitation): ApiCrateOwnerInvitation {
  return {
    crate_id: invite.crate.id,
    crate_name: invite.crate.name,
    invitee_id: invite.invitee.id,
    inviter_id: invite.inviter.id,
    created_at: invite.createdAt,
    expires_at: invite.expiresAt,
  };
}
