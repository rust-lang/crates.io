import { Input } from '@ember/component';
import { uniqueId } from '@ember/helper';
import { on } from '@ember/modifier';
import { LinkTo } from '@ember/routing';

import autoFocus from '@zestia/ember-auto-focus/modifiers/auto-focus';
import perform from 'ember-concurrency/helpers/perform';
import preventDefault from 'ember-event-helpers/helpers/prevent-default';
import eq from 'ember-truth-helpers/helpers/eq';
import not from 'ember-truth-helpers/helpers/not';

import LoadingSpinner from 'crates-io/components/loading-spinner';

<template>
  <form class='form' {{on 'submit' (preventDefault (perform @controller.saveConfigTask))}}>
    <h2>Add a new Trusted Publisher</h2>

    <div class='form-group'>
      {{#let (uniqueId) as |id|}}
        <label for={{id}} class='form-group-name'>Publisher</label>

        <select
          id={{id}}
          disabled={{@controller.saveConfigTask.isRunning}}
          class='publisher-select base-input'
          data-test-publisher
          {{on 'change' @controller.publisherChanged}}
        >
          {{#each @controller.publishers as |publisher|}}
            <option value={{publisher}} selected={{eq @controller.publisher publisher}}>{{publisher}}</option>
          {{/each}}
        </select>
      {{/let}}

      <div class='note'>
        Select the CI/CD platform where your publishing workflow is configured.
      </div>
    </div>

    {{#if (eq @controller.publisher 'GitHub')}}
      <div class='form-group' data-test-namespace-group>
        {{#let (uniqueId) as |id|}}
          <label for={{id}} class='form-group-name'>Repository owner</label>

          <Input
            id={{id}}
            @type='text'
            @value={{@controller.namespace}}
            disabled={{@controller.saveConfigTask.isRunning}}
            aria-required='true'
            aria-invalid={{if @controller.namespaceInvalid 'true' 'false'}}
            class='input base-input'
            data-test-namespace
            {{autoFocus}}
            {{on 'input' @controller.resetNamespaceValidation}}
            {{on 'input' (perform @controller.verifyWorkflowTask)}}
          />

          {{#if @controller.namespaceInvalid}}
            <div class='form-group-error' data-test-error>
              Please enter a repository owner.
            </div>
          {{else}}
            <div class='note'>
              The GitHub organization name or GitHub username that owns the repository.
            </div>
          {{/if}}
        {{/let}}
      </div>

      <div class='form-group' data-test-project-group>
        {{#let (uniqueId) as |id|}}
          <label for={{id}} class='form-group-name'>Repository name</label>

          <Input
            id={{id}}
            @type='text'
            @value={{@controller.project}}
            disabled={{@controller.saveConfigTask.isRunning}}
            aria-required='true'
            aria-invalid={{if @controller.projectInvalid 'true' 'false'}}
            class='input base-input'
            data-test-project
            {{on 'input' @controller.resetProjectValidation}}
            {{on 'input' (perform @controller.verifyWorkflowTask)}}
          />

          {{#if @controller.projectInvalid}}
            <div class='form-group-error' data-test-error>
              Please enter a repository name.
            </div>
          {{else}}
            <div class='note'>
              The name of the GitHub repository that contains the publishing workflow.
            </div>
          {{/if}}
        {{/let}}
      </div>

      <div class='form-group' data-test-workflow-group>
        {{#let (uniqueId) as |id|}}
          <label for={{id}} class='form-group-name'>Workflow filename</label>

          <Input
            id={{id}}
            @type='text'
            @value={{@controller.workflow}}
            disabled={{@controller.saveConfigTask.isRunning}}
            aria-required='true'
            aria-invalid={{if @controller.workflowInvalid 'true' 'false'}}
            class='input base-input'
            data-test-workflow
            {{on 'input' @controller.resetWorkflowValidation}}
            {{on 'input' (perform @controller.verifyWorkflowTask)}}
          />

          {{#if @controller.workflowInvalid}}
            <div class='form-group-error' data-test-error>
              Please enter a workflow filename.
            </div>
          {{else}}
            <div class='note'>
              The filename of the publishing workflow. This file should be present in the
              <code>
                {{#if @controller.repository}}
                  <a
                    href='https://github.com/{{@controller.repository}}/blob/HEAD/.github/workflows/'
                    target='_blank'
                    rel='noopener noreferrer'
                  >.github/workflows/</a>
                {{else}}
                  .github/workflows/
                {{/if}}
              </code>
              directory of the
              {{#if @controller.repository}}<a
                  href='https://github.com/{{@controller.repository}}/'
                  target='_blank'
                  rel='noopener noreferrer'
                >{{@controller.repository}}</a>
              {{/if}}
              repository{{unless @controller.repository ' configured above'}}. For example:
              <code>release.yml</code>
              or
              <code>publish.yml</code>.
            </div>
          {{/if}}

          {{#if (not @controller.verificationUrl)}}
            <div class='workflow-verification' data-test-workflow-verification='initial'>
              The workflow filename will be verified once all necessary fields are filled.
            </div>
          {{else if (eq @controller.verifyWorkflowTask.last.value 'success')}}
            <div class='workflow-verification workflow-verification--success' data-test-workflow-verification='success'>
              ✓ Workflow file found at
              <a href='{{@controller.verificationUrl}}' target='_blank' rel='noopener noreferrer'>
                {{@controller.verificationUrl}}
              </a>
            </div>
          {{else if (eq @controller.verifyWorkflowTask.last.value 'not-found')}}
            <div
              class='workflow-verification workflow-verification--warning'
              data-test-workflow-verification='not-found'
            >
              ⚠ Workflow file not found at
              <a href='{{@controller.verificationUrl}}' target='_blank' rel='noopener noreferrer'>
                {{@controller.verificationUrl}}
              </a>
            </div>
          {{else if (eq @controller.verifyWorkflowTask.last.value 'error')}}
            <div class='workflow-verification workflow-verification--warning' data-test-workflow-verification='error'>
              ⚠ Could not verify workflow file at
              <a href='{{@controller.verificationUrl}}' target='_blank' rel='noopener noreferrer'>
                {{@controller.verificationUrl}}
              </a>
              (network error)
            </div>
          {{else}}
            <div class='workflow-verification' data-test-workflow-verification='verifying'>
              Verifying...
            </div>
          {{/if}}
        {{/let}}
      </div>

      <div class='form-group' data-test-environment-group>
        {{#let (uniqueId) as |id|}}
          <label for={{id}} class='form-group-name'>Environment name (optional)</label>

          <Input
            id={{id}}
            @type='text'
            @value={{@controller.environment}}
            disabled={{@controller.saveConfigTask.isRunning}}
            class='input base-input'
            data-test-environment
          />

          <div class='note'>
            The name of the
            <a
              href='https://docs.github.com/en/actions/deployment/targeting-different-environments/using-environments-for-deployment'
            >GitHub Actions environment</a>
            that the above workflow uses for publishing. This should be configured in the repository settings. A
            dedicated publishing environment is not required, but is
            <strong>strongly recommended</strong>, especially if your repository has maintainers with commit access who
            should not have crates.io publishing access.
          </div>
        {{/let}}
      </div>
    {{else if (eq @controller.publisher 'GitLab')}}
      <div class='form-group' data-test-namespace-group>
        {{#let (uniqueId) as |id|}}
          <label for={{id}} class='form-group-name'>Namespace</label>

          <Input
            id={{id}}
            @type='text'
            @value={{@controller.namespace}}
            disabled={{@controller.saveConfigTask.isRunning}}
            aria-required='true'
            aria-invalid={{if @controller.namespaceInvalid 'true' 'false'}}
            class='input base-input'
            data-test-namespace
            {{autoFocus}}
            {{on 'input' @controller.resetNamespaceValidation}}
          />

          {{#if @controller.namespaceInvalid}}
            <div class='form-group-error' data-test-error>
              Please enter a namespace.
            </div>
          {{else}}
            <div class='note'>
              The GitLab group name or GitLab username that owns the project.
            </div>
          {{/if}}
        {{/let}}
      </div>

      <div class='form-group' data-test-project-group>
        {{#let (uniqueId) as |id|}}
          <label for={{id}} class='form-group-name'>Project</label>

          <Input
            id={{id}}
            @type='text'
            @value={{@controller.project}}
            disabled={{@controller.saveConfigTask.isRunning}}
            aria-required='true'
            aria-invalid={{if @controller.projectInvalid 'true' 'false'}}
            class='input base-input'
            data-test-project
            {{on 'input' @controller.resetProjectValidation}}
          />

          {{#if @controller.projectInvalid}}
            <div class='form-group-error' data-test-error>
              Please enter a project name.
            </div>
          {{else}}
            <div class='note'>
              The name of the GitLab project that contains the publishing workflow.
            </div>
          {{/if}}
        {{/let}}
      </div>

      <div class='form-group' data-test-workflow-group>
        {{#let (uniqueId) as |id|}}
          <label for={{id}} class='form-group-name'>Workflow filepath</label>

          <Input
            id={{id}}
            @type='text'
            @value={{@controller.workflow}}
            disabled={{@controller.saveConfigTask.isRunning}}
            aria-required='true'
            aria-invalid={{if @controller.workflowInvalid 'true' 'false'}}
            class='input base-input'
            data-test-workflow
            {{on 'input' @controller.resetWorkflowValidation}}
          />

          {{#if @controller.workflowInvalid}}
            <div class='form-group-error' data-test-error>
              Please enter a workflow filepath.
            </div>
          {{else}}
            <div class='note'>
              The filepath to the GitLab CI configuration file, relative to the
              {{#if @controller.repository}}<a
                  href='https://gitlab.com/{{@controller.repository}}/'
                  target='_blank'
                  rel='noopener noreferrer'
                >{{@controller.repository}}</a>
              {{/if}}
              repository{{unless @controller.repository ' configured above'}}
              root. For example:
              <code>.gitlab-ci.yml</code>
              or
              <code>ci/publish.yml</code>.
            </div>
          {{/if}}
        {{/let}}
      </div>

      <div class='form-group' data-test-environment-group>
        {{#let (uniqueId) as |id|}}
          <label for={{id}} class='form-group-name'>Environment name (optional)</label>

          <Input
            id={{id}}
            @type='text'
            @value={{@controller.environment}}
            disabled={{@controller.saveConfigTask.isRunning}}
            class='input base-input'
            data-test-environment
          />

          <div class='note'>
            The name of the
            <a href='https://docs.gitlab.com/ee/ci/environments/'>GitLab environment</a>
            that the above workflow uses for publishing. This should be configured in the project settings. A dedicated
            publishing environment is not required, but is
            <strong>strongly recommended</strong>, especially if your project has maintainers with merge access who
            should not have crates.io publishing access.
          </div>
        {{/let}}
      </div>
    {{/if}}

    <div class='buttons'>
      <button
        type='submit'
        class='add-button button button--small'
        disabled={{@controller.saveConfigTask.isRunning}}
        data-test-add
      >
        Add

        {{#if @controller.saveConfigTask.isRunning}}
          <LoadingSpinner @theme='light' class='spinner' data-test-spinner />
        {{/if}}
      </button>

      <LinkTo @route='crate.settings.index' class='cancel-button button button--tan button--small' data-test-cancel>
        Cancel
      </LinkTo>
    </div>
  </form>
</template>
