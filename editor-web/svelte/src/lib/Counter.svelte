<script lang="ts">
  let { count = $bindable() }: { count: number } = $props();
  const increment = () => (count += 1);
</script>

<button type="button" class="counter" onclick={increment}>
  Count is {count}
</button>

<style>
  .counter {
    border: 2px solid #0000;
    background-color: #ccc;
    /* border-radius: 8px; */
  }
  .counter:hover {
    border: 2px solid #aaa;
    background-color: #888;
  }
</style>
