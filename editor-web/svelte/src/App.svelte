<script lang="ts">
  import { FluentBundle } from "@fluent/bundle";
  import {
    createSvelteFluent,
    initFluentContext,
    Localized,
  } from "@nubolab-ffwd/svelte-fluent";
  import resourceEn from "./lang/app.ftl";

  import Setup from "./Setup.svelte";

  const bundle = new FluentBundle("en");
  bundle.addResource(resourceEn);
  initFluentContext(() => createSvelteFluent([bundle]));
</script>

<Setup></Setup>
