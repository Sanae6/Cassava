<script lang="ts">
  import { FluentBundle } from "@fluent/bundle";
  import {
    createSvelteFluent,
    initFluentContext,
    Localized,
  } from "@nubolab-ffwd/svelte-fluent";
  // @ts-ignore
  import resourceEn from "./lang/app.en.ftl";

  import { fetchSingleton } from "$lib/utils/rust";
  import { Settings } from "$bindings/Settings";

  import StartupSettings from "./ui/StartupSettings.svelte";
  import Startup from "./ui/Startup.svelte";

  const bundle = new FluentBundle("en");
  bundle.addResource(resourceEn);
  initFluentContext(() => createSvelteFluent([bundle]));

  const settings = await fetchSingleton(Settings, "settings");
  const config = $state(await settings.getConfig());
  const startInSettings = Object.keys(config.romfs_versions).length == 0;

  type Routes = "settings" | "main";
  let ui: Routes = $state(startInSettings ? "settings" : "main");
</script>

{#if ui == "settings"}
  <StartupSettings
    onreturn={() => {
      ui = "main";
    }}
  />
{:else if ui === "main"}
  <Startup />
{/if}
