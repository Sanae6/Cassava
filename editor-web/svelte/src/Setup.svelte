<script lang="ts">
  import { FluentBundle } from "@fluent/bundle";
  import {
    createSvelteFluent,
    initFluentContext,
    Localized,
  } from "@nubolab-ffwd/svelte-fluent";
  import resourceEn from "./lang/app.ftl";
  import { fetchSingleton, formatVersion } from "./lib/utils/rust";
  import { Settings } from "$bindings/Settings";
    import type { Version } from "$bindings/Version";

  const settings = await fetchSingleton(Settings, "settings");
  const config = await settings.getConfig();
  console.log();
  let path: Partial<Record<Version, string>> = $state(config.romfs_versions);
  $inspect(path);
  let loading = $state(false);
  let progressValue = $state(0);
  let progressMax = $state(1);
</script>

<main>
  <div>
    <div class="text-3xl"><Localized id="app-name" /></div>
    <div><Localized id="setup-romfs-request" /></div>
    <button
      disabled={loading}
      onclick={async () => {
        let romfsDir = await settings.openDirectoryDialog();
        if (romfsDir) {
          loading = true;
          let report = await settings.matchHashes(romfsDir, (processed, total) => {
            progressMax = total;
            progressValue = processed;
          });
          console.log("hello hashes", report);
          loading = false;
          if (report == "NotRomfs") {
            alert("provided directory is not a smo romfs");
          } else if (report == "MixedRomfs") {
            alert("romfs version could not be determined, did you mix romfs versions?")
          } else if ("Version" in report) {
            let version = report["Version"];
            path[version] = romfsDir;
            const result = await settings.setRomfsDirectory(version, romfsDir);
            if ("Err" in result) {
              alert(`failed to save config: ${result["Err"]}`)
            }
          } else if ("VersionInconclusive" in report) {
            alert(`your romfs contains files from multiple versions (${report["VersionInconclusive"].map(formatVersion).join(", ")}).\na clean romfs must be provided`)
          }
        }
      }}
    >
      <Localized id="setup-romfs-open-directory" />
    </button>
    {#if loading}
      <br />
      Hashing provided directory:
      <progress id="file" value={progressValue} max={progressMax}>
        Checking RomFS hashes
      </progress>
    {/if}
    <br />
    <table>
      <tbody>
        <tr>
          <td>1.0.0 directory: </td>
          <td>{path["v100"] ?? ""}</td>
        </tr>
        <tr>
          <td>1.0.1 directory: </td>
          <td>{path["v101"] ?? ""}</td>
        </tr>
        <tr>
          <td>1.1.0 directory: </td>
          <td>{path["v110"] ?? ""}</td>
        </tr>
        <tr>
          <td>1.2.0 directory: </td>
          <td>{path["v120"] ?? ""}</td>
        </tr>
        <tr>
          <td>1.3.0 directory: </td>
          <td>{path["v130"] ?? ""}</td>
        </tr>
      </tbody>
    </table>
  </div>
</main>

<style>
  main {
    max-width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    justify-self: center; /* keeps contents minimally sized inside center of main */
    justify-content: space-evenly;
    align-content: space-evenly;
    text-align: center;
  }
  input {
    /* display: none; */
    content: "burger";
  }
  table {
    border-collapse: separate;
    border-spacing: 1em 0;
  }
</style>
