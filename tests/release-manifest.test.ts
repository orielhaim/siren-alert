import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
const workflow = readFileSync(".github/workflows/release.yml", "utf8");
const definition = workflow.match(
  /function Test-UpdaterAssetUrl \{[\s\S]*?\n {10}\}/,
)?.[0];
if (!definition)
  throw new Error("Release asset validator missing from workflow");
const installer = {
  name: "Siren.Alert_0.1.0_x64-setup.exe",
  apiUrl:
    "https://api.github.com/repos/orielhaim/siren-alert/releases/assets/732856287",
  url: "https://github.com/orielhaim/siren-alert/releases/download/untagged-draft/Siren.Alert_0.1.0_x64-setup.exe",
};
const signature = {
  name: installer.name + ".sig",
  apiUrl:
    "https://api.github.com/repos/orielhaim/siren-alert/releases/assets/732856376",
  url: installer.url + ".sig",
};
for (const [url, expected] of [
  [installer.apiUrl, true],
  [installer.url, true],
  [
    "https://api.github.com/repos/orielhaim/siren-alert/releases/assets/999999999",
    false,
  ],
  [
    "https://api.github.com/repos/another/repository/releases/assets/732856287",
    false,
  ],
  [signature.apiUrl, false],
  [installer.apiUrl + "?redirect=elsewhere", false],
  [installer.url.replace("https:", "http:"), false],
] satisfies [string, boolean][]) {
  test.skipIf(process.platform !== "win32")(
    `release validation ${expected ? "accepts" : "rejects"} ${url}`,
    async () => {
      const script = `${definition}\n$fixture=$env:RELEASE_VALIDATION_FIXTURE | ConvertFrom-Json\nif(Test-UpdaterAssetUrl -Url $fixture.url -Assets $fixture.assets){Write-Output 'accepted'}else{Write-Output 'rejected'}`;
      const child = Bun.spawn(["pwsh", "-NoProfile", "-Command", script], {
        env: {
          ...process.env,
          RELEASE_VALIDATION_FIXTURE: JSON.stringify({
            url,
            assets: [installer, signature],
          }),
        },
        stdout: "pipe",
        stderr: "pipe",
      });
      const [exit, output, errors] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
      ]);
      expect(errors).toBe("");
      expect(exit).toBe(0);
      expect(output.trim()).toBe(expected ? "accepted" : "rejected");
    },
  );
}
