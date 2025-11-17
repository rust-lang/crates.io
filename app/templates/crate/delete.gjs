import { Input } from '@ember/component';
import { on } from '@ember/modifier';

import perform from 'ember-concurrency/helpers/perform';
import preventDefault from 'ember-event-helpers/helpers/prevent-default';
import not from 'ember-truth-helpers/helpers/not';
import or from 'ember-truth-helpers/helpers/or';

import Alert from 'crates-io/components/alert';
import LoadingSpinner from 'crates-io/components/loading-spinner';

<template>
  <div class='wrapper'>
    <form class='content' {{on 'submit' (preventDefault (perform @controller.deleteTask))}}>
      <h1 class='title' data-test-title>Delete the {{@model.name}} crate?</h1>

      <p>Are you sure you want to delete the crate "{{@model.name}}"?</p>

      <Alert @variant='warning'>
        <strong>Important:</strong>
        This action will permanently delete the crate and its associated versions. Deleting a crate cannot be reversed!
      </Alert>

      <div class='impact'>
        <h3>Potential Impact:</h3>
        <ul>
          <li>Users will no longer be able to download this crate.</li>
          <li>Any dependencies or projects relying on this crate will be broken.</li>
          <li>Deleted crates cannot be restored.</li>
        </ul>
      </div>

      <div class='requirements'>
        <h3>Requirements:</h3>
        <p>
          A crate can only be deleted if it is not depended upon by any other crate on crates.io.
        </p>
        <p>Additionally, a crate can only be deleted if either:</p>
        <ol class='first'>
          <li>the crate has been published for less than 72 hours</li>
        </ol>
        <div class='or'>or</div>
        <ol start='2' class='second'>
          <li>
            <ol>
              <li>the crate only has a single owner, <em>and</em></li>
              <li>the crate has been downloaded less than 1000 times for each month it has been published.</li>
            </ol>
          </li>
        </ol>
      </div>

      <div class='reason'>
        <h3>Reason:</h3>
        <label>
          <p>Please tell us why you are deleting this crate:</p>
          <Input
            @type='text'
            @value={{@controller.reason}}
            required={{true}}
            class='reason-input base-input'
            data-test-reason
          />
        </label>
      </div>

      <Alert @variant='warning' @hideIcon={{true}}>
        <label class='confirmation'>
          <Input
            @type='checkbox'
            @checked={{@controller.isConfirmed}}
            disabled={{@controller.deleteTask.isRunning}}
            data-test-confirmation-checkbox
            {{on 'change' @controller.toggleConfirmation}}
          />
          I understand that deleting this crate is permanent and cannot be undone.
        </label>
      </Alert>

      <div class='actions'>
        <button
          type='submit'
          disabled={{or (not @controller.isConfirmed) @controller.deleteTask.isRunning}}
          class='button button--red'
          data-test-delete-button
        >
          Delete this crate
        </button>
        {{#if @controller.deleteTask.isRunning}}
          <div class='spinner-wrapper'>
            <LoadingSpinner class='spinner' data-test-spinner />
          </div>
        {{/if}}
      </div>
    </form>
  </div>
</template>
