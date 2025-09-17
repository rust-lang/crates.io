import { Input } from '@ember/component';
import { fn, uniqueId } from '@ember/helper';
import { on } from '@ember/modifier';
import { LinkTo } from '@ember/routing';

import autoFocus from '@zestia/ember-auto-focus/modifiers/auto-focus';
import perform from 'ember-concurrency/helpers/perform';
import preventDefault from 'ember-event-helpers/helpers/prevent-default';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import { eq, not } from 'ember-truth-helpers';

import LoadingSpinner from 'crates-io/components/loading-spinner';
import PatternDescription from 'crates-io/components/token-scopes/pattern-description';

<template>
  <h2>New API Token</h2>

  <form class='form' {{on 'submit' (preventDefault (perform @controller.saveTokenTask))}}>
    <div class='form-group' data-test-name-group>
      {{#let (uniqueId) as |id|}}
        <label for={{id}} class='form-group-name'>Name</label>

        <Input
          id={{id}}
          @type='text'
          @value={{@controller.name}}
          disabled={{@controller.saveTokenTask.isRunning}}
          autocomplete='off'
          aria-required='true'
          aria-invalid={{if @controller.nameInvalid 'true' 'false'}}
          class='name-input base-input'
          data-test-name
          {{autoFocus}}
          {{on 'input' @controller.resetNameValidation}}
        />

        {{#if @controller.nameInvalid}}
          <div class='form-group-error' data-test-error>
            Please enter a name for this token.
          </div>
        {{/if}}
      {{/let}}
    </div>

    <div class='form-group' data-test-expiry-group>
      {{#let (uniqueId) as |id|}}
        <label for={{id}} class='form-group-name'>Expiration</label>
      {{/let}}

      <div class='select-group'>
        {{#let (uniqueId) as |id|}}
          <select
            id={{id}}
            disabled={{@controller.saveTokenTask.isRunning}}
            class='expiry-select base-input'
            data-test-expiry
            {{on 'change' @controller.updateExpirySelection}}
          >
            <option value='none'>No expiration</option>
            <option value='7'>7 days</option>
            <option value='30'>30 days</option>
            <option value='60'>60 days</option>
            <option value='90' selected>90 days</option>
            <option value='365'>365 days</option>
            <option value='custom'>Custom...</option>
          </select>
        {{/let}}

        {{#if (eq @controller.expirySelection 'custom')}}
          <Input
            @type='date'
            @value={{@controller.expiryDateInput}}
            min={{@controller.today}}
            disabled={{@controller.saveTokenTask.isRunning}}
            aria-invalid={{if @controller.expiryDateInvalid 'true' 'false'}}
            aria-label='Custom expiration date'
            class='expiry-date-input base-input'
            data-test-expiry-date
            {{on 'input' @controller.resetExpiryDateValidation}}
          />
        {{else}}
          <span class='expiry-description' data-test-expiry-description>
            {{@controller.expiryDescription}}
          </span>
        {{/if}}
      </div>
    </div>

    <div class='form-group' data-test-scopes-group>
      <div class='form-group-name'>
        Scopes

        <a
          href='https://rust-lang.github.io/rfcs/2947-crates-io-token-scopes.html'
          target='_blank'
          rel='noopener noreferrer'
          class='help-link'
        >
          <span class='sr-only'>Help</span>
          {{svgJar 'circle-question'}}
        </a>
      </div>

      <ul role='list' class='scopes-list {{if @controller.scopesInvalid "invalid"}}'>
        {{#each @controller.ENDPOINT_SCOPES as |scope|}}
          <li>
            <label data-test-scope={{scope}}>
              <Input
                @type='checkbox'
                @checked={{@controller.isScopeSelected scope}}
                disabled={{@controller.saveTokenTask.isRunning}}
                {{on 'change' (fn @controller.toggleScope scope)}}
              />

              <span class='scope-id'>{{scope}}</span>
              <span class='scope-description'>{{@controller.scopeDescription scope}}</span>
            </label>
          </li>
        {{/each}}
      </ul>

      {{#if @controller.scopesInvalid}}
        <div class='form-group-error' data-test-error>
          Please select at least one token scope.
        </div>
      {{/if}}
    </div>

    <div class='form-group' data-test-scopes-group>
      <div class='form-group-name'>
        Crates

        <a
          href='https://rust-lang.github.io/rfcs/2947-crates-io-token-scopes.html'
          target='_blank'
          rel='noopener noreferrer'
          class='help-link'
        >
          <span class='sr-only'>Help</span>
          {{svgJar 'circle-question'}}
        </a>
      </div>

      <ul role='list' class='crates-list'>
        {{#each @controller.crateScopes as |pattern index|}}
          <li class='crates-scope {{if pattern.showAsInvalid "invalid"}}' data-test-crate-pattern={{index}}>
            <div>
              <Input
                @value={{pattern.pattern}}
                aria-label='Crate name pattern'
                {{on 'input' pattern.resetValidation}}
                {{on 'blur' pattern.validate}}
              />

              <span class='pattern-description' data-test-description>
                {{#if (not pattern.pattern)}}
                  Please enter a crate name pattern
                {{else if pattern.isValid}}
                  <PatternDescription @pattern={{pattern.pattern}} />
                {{else}}
                  Invalid crate name pattern
                {{/if}}
              </span>
            </div>

            <button type='button' data-test-remove {{on 'click' (fn @controller.removeCrateScope index)}}>
              <span class='sr-only'>Remove pattern</span>
              {{svgJar 'trash'}}
            </button>
          </li>
        {{else}}
          <li class='crates-unrestricted' data-test-crates-unrestricted>
            <strong>Unrestricted</strong>
            – This token can be used for all of your crates.
          </li>
        {{/each}}

        <li class='crates-pattern-button'>
          <button type='button' data-test-add-crate-pattern {{on 'click' (fn @controller.addCratePattern '')}}>
            Add pattern
          </button>
        </li>
      </ul>
    </div>

    <div class='buttons'>
      <button
        type='submit'
        class='generate-button button button--small'
        disabled={{@controller.saveTokenTask.isRunning}}
        data-test-generate
      >
        Generate Token

        {{#if @controller.saveTokenTask.isRunning}}
          <LoadingSpinner @theme='light' class='spinner' data-test-spinner />
        {{/if}}
      </button>

      <LinkTo @route='settings.tokens.index' class='cancel-button button button--tan button--small' data-test-cancel>
        Cancel
      </LinkTo>
    </div>

  </form>
</template>
