import { existsSync, readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

type JsonObject = Record<string, unknown>;

function readJson(relativePath: string): JsonObject {
  return JSON.parse(
    readFileSync(new URL(relativePath, import.meta.url), "utf8"),
  ) as JsonObject;
}

const config = readJson("./tauri.conf.json");
const bundle = config.bundle as JsonObject;
const plugins = config.plugins as JsonObject;
const updater = plugins.updater as JsonObject;
const windows = bundle.windows as JsonObject;
const nsis = windows.nsis as JsonObject;
const webviewInstallMode = windows.webviewInstallMode as JsonObject;
const unsignedConfig = readJson("./tauri.windows-unsigned.conf.json");
const unsignedBundle = unsignedConfig.bundle as JsonObject;
const signingExample = readJson("./tauri.windows-signing.conf.example.json");
const signingWindows = (signingExample.bundle as JsonObject).windows as JsonObject;
const packageJson = readJson("../package.json");

describe("Windows bundle configuration", () => {
  it("keeps the release version aligned", () => {
    const version = packageJson.version;

    expect(config.version).toBe(version);
    expect(
      readFileSync(new URL("./Cargo.toml", import.meta.url), "utf8"),
    ).toContain(`version = "${version}"`);
  });

  it("provides a deserializable updater plugin configuration", () => {
    expect(updater.endpoints).toEqual([]);
    expect(updater.pubkey).toBe("");
  });

  it("registers the database before setup can create webviews", () => {
    const source = readFileSync(new URL("./src/lib.rs", import.meta.url), "utf8");
    const managedDatabase = source.indexOf(".manage(database)");
    const setupHook = source.indexOf(".setup(|app|");

    expect(managedDatabase).toBeGreaterThan(-1);
    expect(setupHook).toBeGreaterThan(managedDatabase);
    expect(source).not.toContain("app.manage(database)");
  });

  it("uses the standard NSIS flow with directory and shortcut choices", () => {
    expect(bundle.active).toBe(true);
    expect(bundle.targets).toEqual(["nsis"]);
    expect(nsis.installMode).toBe("currentUser");
    expect(nsis.languages).toEqual(["SimpChinese", "English"]);
    expect(nsis.displayLanguageSelector).toBe(true);
    expect(nsis.startMenuFolder).toBe("抵达 Focus");
    expect(nsis).not.toHaveProperty("template");
  });

  it("bootstraps the evergreen WebView2 runtime when required", () => {
    expect(webviewInstallMode).toEqual({
      type: "downloadBootstrapper",
      silent: true,
    });
  });

  it("keeps Authenticode credentials in a local release overlay", () => {
    expect(windows).not.toHaveProperty("certificateThumbprint");
    expect(signingWindows.certificateThumbprint).toBe(
      "REPLACE_WITH_CERTIFICATE_SHA1_THUMBPRINT",
    );
    expect(signingWindows.digestAlgorithm).toBe("sha256");
    expect(signingWindows.timestampUrl).toMatch(/^https:\/\//);

    const scripts = packageJson.scripts as JsonObject;
    expect(scripts["tauri:build:windows:signed"]).toBe(
      "tauri build --features desktop-app --config src-tauri/tauri.windows-signing.conf.json",
    );
    expect(
      readFileSync(new URL("../.gitignore", import.meta.url), "utf8"),
    ).toContain("src-tauri/tauri.windows-signing.conf.json");
  });

  it("provides an unsigned installer build without updater signing", () => {
    expect(unsignedBundle.createUpdaterArtifacts).toBe(false);

    const scripts = packageJson.scripts as JsonObject;
    expect(scripts["tauri:build:windows"]).toBe(
      "tauri build --features desktop-app --config src-tauri/tauri.windows-unsigned.conf.json",
    );
  });

  it("ships the Windows icon used by executables and installers", () => {
    expect(existsSync(new URL("./icons/icon.ico", import.meta.url))).toBe(true);
  });
});
