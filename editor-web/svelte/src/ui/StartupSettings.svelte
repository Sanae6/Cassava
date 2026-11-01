<script lang="ts">
  import { fetchSingleton, formatVersion } from "$lib/utils/rust";
  import { Settings } from "$bindings/Settings";
  import type { Version } from "$bindings/Version";
  import { getFluentContext, Localized } from "@nubolab-ffwd/svelte-fluent";
  import Button from "$lib/ui/Button.svelte";

  const settings = await fetchSingleton(Settings, "settings");
  const config = await settings.getConfig();
  let romfsPaths: Partial<Record<Version, string>> = $state(
    config.romfs_versions,
  );

  let { onreturn }: { onreturn(): any } = $props();

  let loading = $state(false);
  let progressValue = $state(0);
  let progressMax = $state(1);
</script>

<main>
  <div>
    <div class="text-3xl"><Localized id="app-name" /></div>
    <div><Localized id="setup-romfs-request" /></div>
    <Button
      disabled={loading}
      click={async () => {
        let romfsDir = await settings.openDirectoryDialog();
        if (romfsDir) {
          loading = true;
          let report = await settings.matchHashes(
            romfsDir,
            (processed, total) => {
              progressMax = total;
              progressValue = processed;
            },
          );
          console.log("hello hashes", report);
          loading = false;
          if (report == "NotRomfs") {
            alert("provided directory is not a smo romfs");
          } else if (report == "MixedRomfs") {
            alert(
              "romfs version could not be determined, you may want to redump the game",
            );
          } else if ("Version" in report) {
            let version = report["Version"];
            romfsPaths[version] = romfsDir;
            const result = await settings.setRomfsDirectory(version, romfsDir);
            if ("Err" in result) {
              alert(`failed to save config: ${result["Err"]}`);
            }
          } else if ("IncompleteDump" in report) {
            alert(
              "the dump is incomplete, redump the game to ensure you have every file",
            );
          } else if ("VersionInconclusive" in report) {
            alert(
              `your romfs contains files from multiple versions (${report["VersionInconclusive"].map(formatVersion).join(", ")}).\na clean romfs must be provided`,
            );
          } else if ("FailedToHash" in report) {
            let [path, error] = report["FailedToHash"];

            alert(
              getFluentContext().localize("setup-romfs-hash-error", {
                file: path,
                error,
              }),
            );
          }
        }
      }}
    >
      <Localized id="setup-romfs-open-directory" />
    </Button>
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
          <td>{romfsPaths["v100"] ?? ""}</td>
        </tr>
        <tr>
          <td>1.0.1 directory: </td>
          <td>{romfsPaths["v101"] ?? ""}</td>
        </tr>
        <tr>
          <td>1.1.0 directory: </td>
          <td>{romfsPaths["v110"] ?? ""}</td>
        </tr>
        <tr>
          <td>1.2.0 directory: </td>
          <td>{romfsPaths["v120"] ?? ""}</td>
        </tr>
        <tr>
          <td>1.3.0 directory: </td>
          <td>{romfsPaths["v130"] ?? ""}</td>
        </tr>
      </tbody>
    </table>
    <Button disabled={Object.keys(romfsPaths).length == 0} click={onreturn}
      >Continue</Button
    >
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
