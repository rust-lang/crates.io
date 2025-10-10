import { hash } from '@ember/helper';

import DropdownMenuItem from 'crates-io/components/dropdown/menu-item';

<template>
  <@Content ...attributes>
    <ul class='list'>
      {{yield (hash Item=(component DropdownMenuItem))}}
    </ul>
  </@Content>
</template>
