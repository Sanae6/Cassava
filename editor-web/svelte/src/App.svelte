<script lang="ts">
  import Counter from "./lib/Counter.svelte";
  import { fetchSingleton } from "./lib/utils/rust";
  import { TestService } from "$bindings/TestService";

  let count = $state(0);
  const test = await fetchSingleton(TestService, "test");
  let return_value = $derived(await test.double({ value: count }));
</script>

<section id="center">
  <div>
    <h1 class="text-3xl font-bold">Get burgy {return_value}</h1>
  </div>
  <Counter bind:count />
</section>

<style>
</style>
