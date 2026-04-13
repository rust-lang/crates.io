<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { createClient } from '@crates-io/api-client';

  import Alert from '$lib/components/Alert.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import WorkflowVerification from '$lib/components/WorkflowVerification.svelte';
  import { getNotifications } from '$lib/notifications.svelte';

  type Publisher = 'GitHub' | 'GitLab';

  interface PrefillData {
    publisher: Publisher;
    namespace: string;
    project: string;
  }

  let { data } = $props();

  let notifications = getNotifications();
  let client = createClient({ fetch });

  let crate_id = $derived(data.crate.id);
  let crateName = $derived(data.crate.name);

  // svelte-ignore state_referenced_locally
  let prefill = parseRepositoryUrl(data.crate.repository ?? '');

  let publisher = $state<Publisher>(prefill?.publisher ?? 'GitHub');
  let namespace = $state(prefill?.namespace ?? '');
  let project = $state(prefill?.project ?? '');
  let workflow = $state('');
  let environment = $state('');

  let namespaceInvalid = $state(false);
  let projectInvalid = $state(false);
  let workflowInvalid = $state(false);

  let isSaving = $state(false);

  let repository = $derived(namespace && project ? `${namespace}/${project}` : '');

  let verificationUrl = $derived.by(() => {
    if (publisher !== 'GitHub') return '';
    if (!namespace || !project || !workflow) return '';
    return `https://raw.githubusercontent.com/${namespace}/${project}/HEAD/.github/workflows/${workflow}`;
  });

  function parseRepositoryUrl(repository: string): PrefillData | null {
    if (repository.startsWith('https://github.com/')) {
      return parseGitHubUrl(repository);
    } else if (repository.startsWith('https://gitlab.com/')) {
      return parseGitLabUrl(repository);
    }
    return null;
  }

  function parseGitHubUrl(repository: string): PrefillData | null {
    try {
      let url = new URL(repository);
      let pathParts = url.pathname.slice(1).split('/');
      if (pathParts.length >= 2) {
        return {
          publisher: 'GitHub',
          namespace: pathParts[0]!,
          project: pathParts[1]!.replace(/.git$/, ''),
        };
      }
    } catch {
      // ignore malformed URLs
    }
    return null;
  }

  function parseGitLabUrl(repository: string): PrefillData | null {
    try {
      let url = new URL(repository);
      let pathParts = url.pathname.slice(1).split('/');

      // Find the repository path end (indicated by /-/ for trees/blobs/etc)
      let repoEndIndex = pathParts.indexOf('-');
      if (repoEndIndex !== -1) {
        pathParts = pathParts.slice(0, repoEndIndex);
      }

      if (pathParts.length >= 2) {
        // For GitLab, support nested groups: https://gitlab.com/a/b/c
        // namespace = "a/b", project = "c"
        return {
          publisher: 'GitLab',
          namespace: pathParts.slice(0, -1).join('/'),
          project: pathParts.at(-1)!.replace(/.git$/, ''),
        };
      }
    } catch {
      // ignore malformed URLs
    }
    return null;
  }

  function validate(): boolean {
    namespaceInvalid = !namespace;
    projectInvalid = !project;
    workflowInvalid = !workflow;
    return !namespaceInvalid && !projectInvalid && !workflowInvalid;
  }

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();

    if (!validate()) return;

    isSaving = true;

    try {
      let result;
      if (publisher === 'GitHub') {
        let config = {
          crate: crateName,
          repository_owner: namespace,
          repository_name: project,
          workflow_filename: workflow,
          environment: environment || null,
        };

        result = await client.POST('/api/v1/trusted_publishing/github_configs', { body: { github_config: config } });
      } else {
        let config = {
          crate: crateName,
          namespace,
          project,
          workflow_filepath: workflow,
          environment: environment || null,
        };

        result = await client.POST('/api/v1/trusted_publishing/gitlab_configs', { body: { gitlab_config: config } });
      }

      if (!result.response.ok) {
        let detail = (result.error as unknown as { errors?: { detail?: string }[] })?.errors?.[0]?.detail;
        throw new Error(detail ?? '');
      }

      notifications.success('Trusted Publishing configuration added successfully');
      await goto(resolve('/crates/[crate_id]/settings', { crate_id }));
    } catch (error) {
      let message = 'An error has occurred while adding the Trusted Publishing configuration';
      if (error instanceof Error && error.message) {
        message += `: ${error.message}`;
      }
      notifications.error(message);
    } finally {
      isSaving = false;
    }
  }
</script>

<PageTitle title="Add Trusted Publisher" />

<form class="form" onsubmit={handleSubmit}>
  <h2>Add a new Trusted Publisher</h2>

  <div class="form-group">
    <label for="publisher" class="form-group-name">Publisher</label>

    <select
      id="publisher"
      disabled={isSaving}
      class="publisher-select base-input"
      data-test-publisher
      bind:value={publisher}
    >
      <option value="GitHub">GitHub</option>
      <option value="GitLab">GitLab</option>
    </select>

    <div class="note">Select the CI/CD platform where your publishing workflow is configured.</div>
  </div>

  {#if publisher === 'GitLab'}
    <div class="gitlab-wip-notice">
      <Alert variant="warning" data-test-gitlab-wip-notice>
        GitLab Trusted Publishing is currently in public beta. You may encounter unexpected behavior.
      </Alert>
    </div>
  {/if}

  {#if publisher === 'GitHub'}
    <div class="form-group" data-test-namespace-group>
      <label for="namespace" class="form-group-name">Repository owner</label>

      <!-- svelte-ignore a11y_autofocus -->
      <input
        id="namespace"
        type="text"
        bind:value={namespace}
        disabled={isSaving}
        aria-required="true"
        aria-invalid={namespaceInvalid}
        class="input base-input"
        data-test-namespace
        autofocus
        oninput={() => (namespaceInvalid = false)}
      />

      {#if namespaceInvalid}
        <div class="form-group-error" data-test-error>Please enter a repository owner.</div>
      {:else}
        <div class="note">The GitHub organization name or GitHub username that owns the repository.</div>
      {/if}
    </div>

    <div class="form-group" data-test-project-group>
      <label for="project" class="form-group-name">Repository name</label>

      <input
        id="project"
        type="text"
        bind:value={project}
        disabled={isSaving}
        aria-required="true"
        aria-invalid={projectInvalid}
        class="input base-input"
        data-test-project
        oninput={() => (projectInvalid = false)}
      />

      {#if projectInvalid}
        <div class="form-group-error" data-test-error>Please enter a repository name.</div>
      {:else}
        <div class="note">The name of the GitHub repository that contains the publishing workflow.</div>
      {/if}
    </div>

    <div class="form-group" data-test-workflow-group>
      <label for="workflow" class="form-group-name">Workflow filename</label>

      <input
        id="workflow"
        type="text"
        bind:value={workflow}
        disabled={isSaving}
        aria-required="true"
        aria-invalid={workflowInvalid}
        class="input base-input"
        data-test-workflow
        oninput={() => (workflowInvalid = false)}
      />

      {#if workflowInvalid}
        <div class="form-group-error" data-test-error>Please enter a workflow filename.</div>
      {:else}
        <div class="note" data-test-note>
          The filename of the publishing workflow. This file should be present in the
          <code>
            {#if repository}
              <a
                href="https://github.com/{repository}/blob/HEAD/.github/workflows/"
                target="_blank"
                rel="noopener noreferrer">.github/workflows/</a
              >
            {:else}
              .github/workflows/
            {/if}
          </code>
          directory of the
          {#if repository}
            <a href="https://github.com/{repository}/" target="_blank" rel="noopener noreferrer">{repository}</a>
          {/if}
          repository{repository ? '' : ' configured above'}. For example:
          <code>release.yml</code>
          or
          <code>publish.yml</code>.
        </div>
      {/if}

      <WorkflowVerification url={verificationUrl} fieldType="filename" />
    </div>

    <div class="form-group" data-test-environment-group>
      <label for="environment" class="form-group-name">Environment name (optional)</label>

      <input
        id="environment"
        type="text"
        bind:value={environment}
        disabled={isSaving}
        class="input base-input"
        data-test-environment
      />

      <div class="note">
        The name of the
        <a
          href="https://docs.github.com/en/actions/deployment/targeting-different-environments/using-environments-for-deployment"
          >GitHub Actions environment</a
        >
        that the above workflow uses for publishing. This should be configured in the repository settings. A dedicated publishing
        environment is not required, but is
        <strong>strongly recommended</strong>, especially if your repository has maintainers with commit access who
        should not have crates.io publishing access.
      </div>
    </div>
  {:else if publisher === 'GitLab'}
    <div class="form-group" data-test-namespace-group>
      <label for="namespace" class="form-group-name">Namespace</label>

      <!-- svelte-ignore a11y_autofocus -->
      <input
        id="namespace"
        type="text"
        bind:value={namespace}
        disabled={isSaving}
        aria-required="true"
        aria-invalid={namespaceInvalid}
        class="input base-input"
        data-test-namespace
        autofocus
        oninput={() => (namespaceInvalid = false)}
      />

      {#if namespaceInvalid}
        <div class="form-group-error" data-test-error>Please enter a namespace.</div>
      {:else}
        <div class="note">The GitLab group name or GitLab username that owns the project.</div>
      {/if}
    </div>

    <div class="form-group" data-test-project-group>
      <label for="project" class="form-group-name">Project</label>

      <input
        id="project"
        type="text"
        bind:value={project}
        disabled={isSaving}
        aria-required="true"
        aria-invalid={projectInvalid}
        class="input base-input"
        data-test-project
        oninput={() => (projectInvalid = false)}
      />

      {#if projectInvalid}
        <div class="form-group-error" data-test-error>Please enter a project name.</div>
      {:else}
        <div class="note">The name of the GitLab project that contains the publishing workflow.</div>
      {/if}
    </div>

    <div class="form-group" data-test-workflow-group>
      <label for="workflow" class="form-group-name">Workflow filepath</label>

      <input
        id="workflow"
        type="text"
        bind:value={workflow}
        disabled={isSaving}
        aria-required="true"
        aria-invalid={workflowInvalid}
        class="input base-input"
        data-test-workflow
        oninput={() => (workflowInvalid = false)}
      />

      {#if workflowInvalid}
        <div class="form-group-error" data-test-error>Please enter a workflow filepath.</div>
      {:else}
        <div class="note" data-test-note>
          The filepath to the GitLab CI configuration file, relative to the root of the
          {#if repository}
            <a href="https://gitlab.com/{repository}/" target="_blank" rel="noopener noreferrer">{repository}</a>
          {/if}
          repository{repository ? '' : ' configured above'}. For example:
          <code>.gitlab-ci.yml</code>
          or
          <code>ci/publish.yml</code>.
        </div>
      {/if}
    </div>

    <div class="form-group" data-test-environment-group>
      <label for="environment" class="form-group-name">Environment name (optional)</label>

      <input
        id="environment"
        type="text"
        bind:value={environment}
        disabled={isSaving}
        class="input base-input"
        data-test-environment
      />

      <div class="note">
        The name of the
        <a href="https://docs.gitlab.com/ee/ci/environments/">GitLab environment</a>
        that the above workflow uses for publishing. This should be configured in the project settings. A dedicated publishing
        environment is not required, but is
        <strong>strongly recommended</strong>, especially if your project has maintainers with merge access who should
        not have crates.io publishing access.
      </div>
    </div>
  {/if}

  <div class="buttons">
    <button type="submit" class="add-button button button--small" disabled={isSaving} data-test-add>
      Add

      {#if isSaving}
        <LoadingSpinner theme="light" class="spinner" data-test-spinner />
      {/if}
    </button>

    <a
      href={resolve('/crates/[crate_id]/settings', { crate_id })}
      class="cancel-button button button--tan button--small"
      data-test-cancel
    >
      Cancel
    </a>
  </div>
</form>

<style>
  .form {
    max-width: 600px;
    margin: var(--space-m) auto;
  }

  .form-group,
  .buttons {
    margin: var(--space-m) 0;
  }

  .publisher-select {
    max-width: 600px;
    width: 100%;
    padding-right: var(--space-m);
    background-image: url('$lib/assets/dropdown-black.svg');
    background-repeat: no-repeat;
    background-position: calc(100% - var(--space-2xs)) center;
    background-size: 10px;
    appearance: none;

    :global([data-color-scheme='system']) & {
      @media (prefers-color-scheme: dark) {
        background-image: url('$lib/assets/dropdown-white.svg');
      }
    }

    :global([data-color-scheme='dark']) & {
      background-image: url('$lib/assets/dropdown-white.svg');
    }
  }

  .note {
    margin-top: var(--space-2xs);
    font-size: 0.85em;
  }

  .input {
    max-width: 600px;
    width: 100%;
  }

  .buttons {
    display: flex;
    gap: var(--space-2xs);
    flex-wrap: wrap;
  }

  .add-button {
    border-radius: var(--space-3xs);

    :global(.spinner) {
      margin-left: var(--space-2xs);
    }
  }

  .cancel-button {
    border-radius: var(--space-3xs);
  }

  .gitlab-wip-notice {
    margin: var(--space-m) 0;
    font-size: 0.85em;
  }
</style>
