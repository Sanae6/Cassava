<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    click,
    children,
    disabled,
  }: {
    click(): void,
    children: Snippet,
    disabled: boolean,
  } = $props();


</script>

<button disabled={disabled} onclick={click}>{@render children()}</button>

<style>
  button {
    border: 1px solid gray;
    background-color: #444;
    border-radius: 4px;
    padding: 0 4px;
  }

  button:hover {
    background-color: #555;
  }
  button:active {
    background-color: #666;
  }
</style>
