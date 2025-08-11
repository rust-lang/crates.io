import { service } from '@ember/service';
import Component from '@glimmer/component';

/**
 * A component that wraps elements (probably mostly buttons in practice) that
 * can be used to perform potentially privileged actions.
 *
 * This component requires a `userAuthorised` property, which must be a
 * `boolean` indicating whether the user is authorised for this action. Admin
 * rights need not be taken into account.
 *
 * If the current user is an admin and they have enabled sudo mode, then they
 * are always allowed to perform the action, regardless of the return value of
 * `userAuthorised`.
 *
 * There are three content blocks supported by this component:
 *
 * - `default`: required; this is displayed when the user is authorised to
 *              perform the action.
 * - `placeholder`: this is displayed when the user _could_ be authorised to
 *                  perform the action (that is, they're an admin but have not
 *                  enabled sudo mode), but currently cannot perform the action.
 *                  If omitted, the `default` block is used with all form
 *                  controls disabled and a tooltip added.
 * - `unprivileged`: this is displayed when the user is not able to perform this
 *                   action, and cannot be authorised to do so. If omitted, an
 *                   empty block will be used.
 *
 * Note that all blocks will be output with a wrapping `<div>` for technical
 * reasons, so be sure to style accordingly if necessary.
 */
export default class PrivilegedAction extends Component {
  /** @type {import("../services/session").default} */
  @service session;

  /** @return {boolean} */
  get isPrivileged() {
    return this.session.isSudoEnabled || this.args.userAuthorised;
  }

  /** @return {boolean} */
  get canBePrivileged() {
    return !this.args.userAuthorised && this.session.currentUser?.is_admin && !this.session.isSudoEnabled;
  }
}
