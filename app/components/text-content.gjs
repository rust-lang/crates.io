<template>
  <div ...attributes class='wrapper {{if @boxed "boxed"}}'>
    {{yield}}
  </div>
</template>
