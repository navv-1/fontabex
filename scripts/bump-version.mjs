#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const VERSION_FILES = {
  packageJson: "package.json",
  packageLock: "package-lock.json",
  cargoToml: "src-tauri/Cargo.toml",
  tauriConfig: "src-tauri/tauri.conf.json",
};

const bumpKind = process.argv[2];

function fail(message) {
  console.error(`version: ${message}`);
  process.exit(1);
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function writeJson(path, data) {
  writeFileSync(path, `${JSON.stringify(data, null, 2)}\n`);
}

function readCargoVersion() {
  const cargoToml = readFileSync(VERSION_FILES.cargoToml, "utf8");
  const match = cargoToml.match(/^version = "([^"]+)"$/m);
  if (!match)
    fail(`could not find package version in ${VERSION_FILES.cargoToml}`);
  return match[1];
}

function assertCleanWorkingTree() {
  const status = execFileSync("git", ["status", "--porcelain"], {
    encoding: "utf8",
  });
  if (status.trim()) {
    fail("working tree must be clean before bumping the version");
  }
}

function parseVersion(version) {
  const match = version.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) fail(`unsupported version "${version}"; expected x.y.z`);
  return match.slice(1).map(Number);
}

function nextVersion(version, kind) {
  const [major, minor, patch] = parseVersion(version);

  switch (kind) {
    case "patch":
      return `${major}.${minor}.${patch + 1}`;
    case "minor":
      return `${major}.${minor + 1}.0`;
    case "major":
      return `${major + 1}.0.0`;
    default:
      if (/^\d+\.\d+\.\d+$/.test(kind ?? "")) return kind;
      fail(
        "usage: npm run version:patch|minor|major or node scripts/bump-version.mjs x.y.z",
      );
  }
}

function assertVersionsMatch(versions) {
  const uniqueVersions = new Set(Object.values(versions));
  if (uniqueVersions.size > 1) {
    fail(
      `version files are out of sync: ${Object.entries(versions)
        .map(([name, version]) => `${name}=${version}`)
        .join(", ")}`,
    );
  }
}

assertCleanWorkingTree();

const packageJson = readJson(VERSION_FILES.packageJson);
const packageLock = readJson(VERSION_FILES.packageLock);
const tauriConfig = readJson(VERSION_FILES.tauriConfig);
const cargoVersion = readCargoVersion();

const versions = {
  packageJson: packageJson.version,
  packageLock: packageLock.version,
  packageLockRoot: packageLock.packages?.[""]?.version,
  cargoToml: cargoVersion,
  tauriConfig: tauriConfig.version,
};

assertVersionsMatch(versions);

const currentVersion = packageJson.version;
const version = nextVersion(currentVersion, bumpKind);

packageJson.version = version;
packageLock.version = version;
packageLock.packages[""].version = version;
tauriConfig.version = version;

const cargoToml = readFileSync(VERSION_FILES.cargoToml, "utf8").replace(
  /^version = "[^"]+"$/m,
  `version = "${version}"`,
);

writeJson(VERSION_FILES.packageJson, packageJson);
writeJson(VERSION_FILES.packageLock, packageLock);
writeJson(VERSION_FILES.tauriConfig, tauriConfig);
writeFileSync(VERSION_FILES.cargoToml, cargoToml);

console.log(`version: ${currentVersion} -> ${version}`);
