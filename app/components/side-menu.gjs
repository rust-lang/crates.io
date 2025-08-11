import { hash } from '@ember/helper';

import SideMenuItem from 'crates-io/components/side-menu/item';
<template>
  <ul role='list' ...attributes class='list'>
    {{yield (hash Item=(component SideMenuItem))}}
  </ul>
</template>
