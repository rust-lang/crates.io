import { Input } from '@ember/component';
import { on } from '@ember/modifier';
import { LinkTo } from '@ember/routing';

import perform from 'ember-concurrency/helpers/perform';
import preventDefault from 'ember-event-helpers/helpers/prevent-default';
import pageTitle from 'ember-page-title/helpers/page-title';
import not from 'ember-truth-helpers/helpers/not';
import or from 'ember-truth-helpers/helpers/or';

import CrateHeader from 'crates-io/components/crate-header';
import Tooltip from 'crates-io/components/tooltip';
import TrustpubOnlyCheckbox from 'crates-io/components/trustpub-only-checkbox';
import UserAvatar from 'crates-io/components/user-avatar';

<template>
  {{pageTitle 'Manage Crate Settings'}}

  <CrateHeader @crate={{@controller.crate}} />

  <div class='header'>
    <h2>Owners</h2>
    {{#unless @controller.addOwnerVisible}}
      <button
        type='button'
        class='button button--small'
        data-test-add-owner-button
        {{on 'click' @controller.showAddOwnerForm}}
      >Add Owner</button>
    {{/unless}}
  </div>

  {{#if @controller.addOwnerVisible}}
    <form class='email-form' {{on 'submit' (preventDefault (perform @controller.addOwnerTask))}}>
      <label class='email-input-label' for='new-owner-username'>
        Username
      </label>
      <Input
        @type='text'
        id='new-owner-username'
        @value={{@controller.username}}
        placeholder='Username'
        class='email-input'
        name='username'
      />
      <button
        type='submit'
        disabled={{not @controller.username}}
        class='button button--small'
        data-test-save-button
      >Add</button>
    </form>
  {{/if}}

  <div class='list' data-test-owners>
    {{#each @controller.crate.owner_team as |team|}}
      <div class='row' data-test-owner-team={{team.login}}>
        <LinkTo @route={{team.kind}} @model={{team.login}}>
          <UserAvatar @user={{team}} @size='medium-small' />
        </LinkTo>
        <LinkTo @route={{team.kind}} @model={{team.login}}>
          {{team.display_name}}
        </LinkTo>
        <div class='email-column'>
          {{team.email}}
        </div>
        <button
          type='button'
          class='button button--small'
          data-test-remove-owner-button
          {{on 'click' (perform @controller.removeOwnerTask team)}}
        >Remove</button>
      </div>
    {{/each}}
    {{#each @controller.crate.owner_user as |user|}}
      <div class='row' data-test-owner-user={{user.login}}>
        <LinkTo @route={{user.kind}} @model={{user.login}}>
          <UserAvatar @user={{user}} @size='medium-small' />
        </LinkTo>
        <LinkTo @route={{user.kind}} @model={{user.login}}>
          {{#if user.name}}
            {{user.name}}
          {{else}}
            {{user.login}}
          {{/if}}
        </LinkTo>
        <div class='email-column'>
          {{user.email}}
        </div>
        <button
          type='button'
          class='button button--small'
          data-test-remove-owner-button
          {{on 'click' (perform @controller.removeOwnerTask user)}}
        >Remove</button>
      </div>
    {{/each}}
  </div>

  <div class='header'>
    <h2>Trusted Publishing</h2>
    <div>
      <LinkTo
        @route='docs.trusted-publishing'
        class='button button--tan button--small'
        data-test-trusted-publishing-docs-button
      >
        Learn more
      </LinkTo>
      <LinkTo
        @route='crate.settings.new-trusted-publisher'
        class='button button--small'
        data-test-add-trusted-publisher-button
      >
        Add
      </LinkTo>
    </div>
  </div>

  <div class='trustpub'>
    <table data-test-trusted-publishing>
      <thead>
        <tr>
          <th>Publisher</th>
          <th>Details</th>
          <th><span class='sr-only'>Actions</span></th>
        </tr>
      </thead>
      <tbody>
        {{#each @controller.githubConfigs as |config|}}
          <tr data-test-github-config={{config.id}}>
            <td>GitHub</td>
            <td class='details'>
              <strong>Repository:</strong>
              <a
                href='https://github.com/{{config.repository_owner}}/{{config.repository_name}}'
                target='_blank'
                rel='noopener noreferrer'
              >{{config.repository_owner}}/{{config.repository_name}}</a>
              <span class='owner-id'>
                · Owner ID:
                {{config.repository_owner_id}}
                <Tooltip>
                  This is the owner ID for
                  <strong>{{config.repository_owner}}</strong>
                  from when this configuration was created. If
                  <strong>{{config.repository_owner}}</strong>
                  was recreated on GitHub, this configuration will need to be recreated as well.
                </Tooltip>
              </span><br />
              <strong>Workflow:</strong>
              <a
                href='https://github.com/{{config.repository_owner}}/{{config.repository_name}}/blob/HEAD/.github/workflows/{{config.workflow_filename}}'
                target='_blank'
                rel='noopener noreferrer'
              >{{config.workflow_filename}}</a><br />
              {{#if config.environment}}
                <strong>Environment:</strong>
                {{config.environment}}
              {{/if}}
            </td>
            <td class='actions'>
              <button
                type='button'
                class='button button--small'
                data-test-remove-config-button
                {{on 'click' (perform @controller.removeConfigTask config)}}
              >Remove</button>
            </td>
          </tr>
        {{/each}}
        {{#each @controller.gitlabConfigs as |config|}}
          <tr data-test-gitlab-config={{config.id}}>
            <td>GitLab</td>
            <td class='details'>
              <strong>Repository:</strong>
              <a
                href='https://gitlab.com/{{config.namespace}}/{{config.project}}'
                target='_blank'
                rel='noopener noreferrer'
              >{{config.namespace}}/{{config.project}}</a>
              <span class='owner-id'>
                · Namespace ID:
                {{#if config.namespace_id}}
                  {{config.namespace_id}}
                  <Tooltip>
                    This is the namespace ID for
                    <strong>{{config.namespace}}</strong>
                    from the first publish using this configuration. If
                    <strong>{{config.namespace}}</strong>
                    was recreated on GitLab, this configuration will need to be recreated as well.
                  </Tooltip>
                {{else}}
                  (not yet set)
                  <Tooltip>
                    The namespace ID will be captured from the first publish using this configuration.
                  </Tooltip>
                {{/if}}
              </span><br />
              <strong>Workflow:</strong>
              <a
                href='https://gitlab.com/{{config.namespace}}/{{config.project}}/-/blob/HEAD/{{config.workflow_filepath}}'
                target='_blank'
                rel='noopener noreferrer'
              >{{config.workflow_filepath}}</a><br />
              {{#if config.environment}}
                <strong>Environment:</strong>
                {{config.environment}}
              {{/if}}
            </td>
            <td class='actions'>
              <button
                type='button'
                class='button button--small'
                data-test-remove-config-button
                {{on 'click' (perform @controller.removeConfigTask config)}}
              >Remove</button>
            </td>
          </tr>
        {{/each}}
        {{#unless (or @controller.githubConfigs.length @controller.gitlabConfigs.length)}}
          <tr class='no-trustpub-config' data-test-no-config>
            <td colspan='3'>No trusted publishers configured for this crate.</td>
          </tr>
        {{/unless}}
      </tbody>
    </table>

    {{#if @controller.showTrustpubOnlyCheckbox}}
      <TrustpubOnlyCheckbox @crate={{@controller.crate}} class='trustpub-only-checkbox' />
    {{/if}}
  </div>

  <h2 class='header'>Danger Zone</h2>

  <div>
    <LinkTo @route='crate.delete' class='button button--red' data-test-delete-button>
      Delete this crate
    </LinkTo>
  </div>
</template>
