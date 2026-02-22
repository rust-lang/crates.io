<script lang="ts">
  import { createClient } from '@crates-io/api-client';

  import EmailInput from '$lib/components/EmailInput.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import SettingsPage from '$lib/components/SettingsPage.svelte';
  import UserAvatar from '$lib/components/UserAvatar.svelte';
  import { getNotifications } from '$lib/notifications.svelte';
  import { getSession } from '$lib/utils/session.svelte';

  let session = getSession();
  let notifications = getNotifications();
  let client = createClient({ fetch });

  let user = $derived(session.currentUser!);
  let publishNotifications = $state(session.currentUser!.publish_notifications);
  let isUpdating = $state(false);

  async function updateNotificationSettings() {
    isUpdating = true;
    try {
      let result = await client.PUT('/api/v1/users/{user}', {
        params: { path: { user: user.id } },
        body: { user: { publish_notifications: publishNotifications } },
      });

      if (!result.response.ok) {
        throw new Error();
      }

      user.publish_notifications = publishNotifications;
    } catch {
      notifications.error('Something went wrong while updating your notification settings. Please try again later!');
    } finally {
      isUpdating = false;
    }
  }
</script>

<svelte:head>
  <title>Account Settings | crates.io: Rust Package Registry</title>
</svelte:head>

<PageHeader title="Account Settings" />

<SettingsPage>
  <div class="me-profile">
    <h2>Profile Information</h2>

    <div class="info">
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
      <a href={user.url} title={user.login}>
        <UserAvatar user={{ ...user, kind: 'user' }} size="medium" />
      </a>

      <dl>
        <dt>Name</dt>
        <dd>{user.name}</dd>
        <dt>GitHub Account</dt>
        <dd>{user.login}</dd>
      </dl>
    </div>

    <p>
      To update your name and GitHub account, change them in your GitHub profile, then sign out and login again to
      crates.io. You cannot change these settings directly on crates.io, but we accept whatever values come from GitHub.
    </p>
  </div>

  <div class="me-email">
    <h2>User Email</h2>
    <EmailInput bind:user data-test-email-input />
  </div>

  <div class="notifications" data-test-notifications>
    <h2>Notification Settings</h2>

    <label class="checkbox-input">
      <input type="checkbox" bind:checked={publishNotifications} disabled={isUpdating} />
      <span class="label">Publish Notifications</span>
      <span class="note">
        Publish notifications are sent to your email address whenever new versions of a crate that you own are
        published. These can be useful to quickly detect compromised accounts or API tokens.
      </span>
    </label>

    <div class="buttons">
      <button type="button" class="button button--small" disabled={isUpdating} onclick={updateNotificationSettings}>
        Update preferences
      </button>
      {#if isUpdating}
        <LoadingSpinner data-test-spinner />
      {/if}
    </div>
  </div>
</SettingsPage>

<style>
  .me-profile {
    margin-bottom: var(--space-s);

    .info {
      display: flex;
    }

    dl {
      margin: 0 0 0 var(--space-m);
      line-height: 1.5;
      font-size: 110%;

      dt {
        font-weight: bold;
        width: 150px;
        text-align: right;
        float: left;
        clear: both;
      }

      dd {
        float: left;
        margin-left: var(--space-xs);
      }
    }

    p {
      line-height: 1.5;
    }

    @media only screen and (max-width: 550px) {
      .info :global(img) {
        display: none;
      }
    }
  }

  .me-email {
    margin-bottom: var(--space-m);
    display: flex;
    flex-direction: column;
  }

  .notifications {
    margin-bottom: var(--space-s);
  }

  .checkbox-input {
    display: grid;
    grid-template:
      'checkbox label' auto
      '- note' auto /
      auto 1fr;
    row-gap: var(--space-3xs);
    column-gap: var(--space-xs);
  }

  .label {
    grid-area: label;
    font-weight: bold;
  }

  .note {
    grid-area: note;
    display: block;
    font-size: 85%;
  }

  .buttons {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    margin-top: var(--space-s);
  }
</style>
