{{page-title 'Manage Crate Settings'}}

<CrateHeader @crate={{this.crate}} />

<div class="header">
  <h2>Owners</h2>
  {{#unless this.addOwnerVisible}}
    <button type="button" class="button button--small" data-test-add-owner-button {{on "click" this.showAddOwnerForm}}>Add Owner</button>
  {{/unless}}
</div>

{{#if this.addOwnerVisible}}
  <form class="email-form" {{on "submit" (prevent-default (perform this.addOwnerTask))}}>
    <label class="email-input-label" for='new-owner-username'>
      Username
    </label>
    <Input @type="text" id="new-owner-username" @value={{this.username}} placeholder="Username" class="email-input" name="username" />
    <button type="submit" disabled={{not this.username}} class="button button--small" data-test-save-button>Add</button>
  </form>
{{/if}}

<div class='list' data-test-owners>
  {{#each this.crate.owner_team as |team|}}
    <div class='row' data-test-owner-team={{team.login}}>
      <LinkTo @route={{team.kind}} @model={{team.login}}>
        <UserAvatar @user={{team}} @size="medium-small" />
      </LinkTo>
      <LinkTo @route={{team.kind}} @model={{team.login}}>
        {{team.display_name}}
      </LinkTo>
      <div class="email-column">
        {{team.email}}
      </div>
      <button type="button" class="button button--small" data-test-remove-owner-button {{on "click" (perform this.removeOwnerTask team)}}>Remove</button>
    </div>
  {{/each}}
  {{#each this.crate.owner_user as |user|}}
    <div class='row' data-test-owner-user={{user.login}}>
      <LinkTo @route={{user.kind}} @model={{user.login}}>
        <UserAvatar @user={{user}} @size="medium-small" />
      </LinkTo>
      <LinkTo @route={{user.kind}} @model={{user.login}}>
        {{#if user.name}}
          {{user.name}}
        {{else}}
          {{user.login}}
        {{/if}}
      </LinkTo>
      <div class="email-column">
        {{user.email}}
      </div>
      <button type="button" class="button button--small" data-test-remove-owner-button {{on "click" (perform this.removeOwnerTask user)}}>Remove</button>
    </div>
  {{/each}}
</div>

<div class="header">
  <h2>Trusted Publishing</h2>
  <div>
    <LinkTo @route="docs.trusted-publishing" class="button button--tan button--small" data-test-trusted-publishing-docs-button>
      Learn more
    </LinkTo>
    <LinkTo @route="crate.settings.new-trusted-publisher" class="button button--small" data-test-add-trusted-publisher-button>
      Add
    </LinkTo>
  </div>
</div>

<table class="trustpub" data-test-trusted-publishing>
  <thead>
  <tr>
    <th>Publisher</th>
    <th>Details</th>
    <th><span class="sr-only">Actions</span></th>
  </tr>
  </thead>
  <tbody>
  {{#each this.githubConfigs as |config|}}
    <tr data-test-github-config={{config.id}}>
      <td>GitHub</td>
      <td class="details">
        <strong>Repository:</strong> <a href="https://github.com/{{config.repository_owner}}/{{config.repository_name}}" target="_blank" rel="noopener noreferrer">{{config.repository_owner}}/{{config.repository_name}}</a><br>
        <strong>Workflow:</strong> <a href="https://github.com/{{config.repository_owner}}/{{config.repository_name}}/blob/HEAD/.github/workflows/{{config.workflow_filename}}" target="_blank" rel="noopener noreferrer">{{config.workflow_filename}}</a><br>
        {{#if config.environment}}
          <strong>Environment:</strong> {{config.environment}}
        {{/if}}
      </td>
      <td class="actions">
        <button type="button" class="button button--small" data-test-remove-config-button {{on "click" (perform this.removeConfigTask config)}}>Remove</button>
      </td>
    </tr>
  {{else}}
    <tr data-test-no-config>
      <td colspan="3">No trusted publishers configured for this crate.</td>
    </tr>
  {{/each}}
  </tbody>
</table>

<h2 class="header">Danger Zone</h2>

<div>
  <LinkTo @route="crate.delete" class="button button--red" data-test-delete-button>
    Delete this crate
  </LinkTo>
</div>
