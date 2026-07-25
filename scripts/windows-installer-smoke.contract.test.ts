import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const script = readFileSync(
  new URL("./windows-installer-smoke.ps1", import.meta.url),
  "utf8",
);
const packageJson = JSON.parse(
  readFileSync(new URL("../package.json", import.meta.url), "utf8"),
) as { scripts: Record<string, string> };

describe("Windows installer smoke test", () => {
  it("accepts separate baseline and upgrade NSIS packages", () => {
    expect(script).toContain("[string]$BaselineInstallerPath");
    expect(script).toContain("[string]$UpgradeInstallerPath");
    expect(script).toContain(
      'Invoke-CheckedProcess -FilePath $baselineInstaller -ArgumentList @("/S", "/D=`"$InstallDirectory`"")',
    );
    expect(script).toContain(
      'Invoke-CheckedProcess -FilePath $upgradeInstaller -ArgumentList @("/S", "/D=`"$InstallDirectory`"")',
    );
    expect(script).toContain(
      "must reference NSIS .exe packages",
    );
    expect(script).toContain("$upgradeHash -eq $baselineHash");
  });

  it("probes both installed versions and performs a silent uninstall", () => {
    expect(script.match(/Start-And-ProbeApplication/g)).toHaveLength(3);
    expect(script).toContain(
      'Invoke-CheckedProcess -FilePath $uninstallerPath -ArgumentList @("/S")',
    );
    expect(script).toContain(
      "Wait-ForFileRemoval -Path $executablePath",
    );
    expect(script).toContain(
      "Wait-ForFileRemoval -Path $uninstallerPath",
    );
  });

  it("protects existing user data and verifies preservation", () => {
    expect(script).toContain(
      'if (Test-Path -LiteralPath $AppDataDirectory) {',
    );
    expect(script).toContain(
      'Set-Content -LiteralPath $markerPath -Value $markerValue -NoNewline',
    );
    expect(script).toContain(
      'Test-Path -LiteralPath $databasePath -PathType Leaf',
    );
    expect(script).toContain("dataPreserved = $true");
  });

  it("is exposed through the project scripts", () => {
    expect(packageJson.scripts["smoke:windows-installer"]).toBe(
      "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows-installer-smoke.ps1",
    );
  });
});
