import addEmail from './emails/add.js';
import confirmEmail from './emails/confirm.js';
import deleteEmail from './emails/delete.js';
import resend from './emails/resend.js';
import enableNotifications from './emails/set-primary.js';
import getUser from './users/get.js';
import me from './users/me.js';
import updateUser from './users/update.js';

export default [getUser, updateUser, resend, me, confirmEmail, addEmail, deleteEmail, enableNotifications];
