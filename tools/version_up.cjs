// @ts-check

// Usage:
//   node version_up.js <version_type> [--dry-run]
//
// Version Types:
//   major               Increment major version (e.g. 0.0.0 → 1.0.0)
//   minor               Increment minor version (e.g. 0.0.0 → 0.1.0)
//   patch               Increment patch version (e.g. 0.0.0 → 0.0.1)
//   prerelease          Auto-increment existing prerelease tag (e.g. alpha.7 → alpha.8)
//   prerelease=<tag>    Add or replace prerelease tag (e.g. 1.2.3 → 1.2.3-alpha.0)
//
// Examples:
//   node ./tools/version_up.cjs prerelease           # e.g. 1.2.3-alpha.7 → 1.2.3-alpha.8
//   node ./tools/version_up.cjs prerelease=beta      # e.g. 1.2.3 → 1.2.3-beta.0
//   node ./tools/version_up.cjs minor                # e.g. 1.2.3 → 1.3.0
//   node ./tools/version_up.cjs patch --dry-run      # Simulate patch bump

const fs = require('node:fs');
const path = require('node:path');
const { execSync } = require('node:child_process');

const baseDir = path.resolve(__dirname, '..');
const paths = {
  packageJson: path.join(baseDir, 'package.json'),
  cargoToml: path.join(baseDir, 'Cargo.toml'),
  issueTemplate: path.join(baseDir, '.github', 'ISSUE_TEMPLATE', 'bug-report.yaml'),
};

const args = process.argv.slice(2);
const versionArg = args.find((arg) => !arg.startsWith('--')) || '2'; // default minor
const isDryRun = args.includes('--dry-run');
const useGpg = true;

main();

/**
 * Entry point of the version bumping script.
 * Parses the current version, calculates the new version, updates relevant files,
 * and performs Git commit and tag.
 */
function main() {
  const currentVersion = require(paths.packageJson).version;
  const newVersion = bumpVersion(currentVersion, versionArg);

  log(`Version: ${currentVersion} → ${newVersion}`);
  if (isDryRun) {
    return;
  }

  updateFiles(newVersion);
  commitAndTag(currentVersion, newVersion);
}

/**
 * Bumps the version string based on the specified release type.
 * Supports "major", "minor", "patch", "prerelease", and "prerelease=tag".
 *
 * @param {string} current - The current version (e.g., "1.2.3-alpha.7").
 * @param {string} type - Bump type: "major", "minor", "patch", "prerelease", or "prerelease=tag".
 * @returns {string} The new version.
 */
function bumpVersion(current, type) {
  const match = current.match(/^(\d+)\.(\d+)\.(\d+)(?:-([\w.-]+))?$/);
  if (!match) {
    throw new Error(`Invalid version format: ${current}`);
  }

  const major = +match[1];
  const minor = +match[2];
  const patch = +match[3];
  const pre = match[4];

  if (type.startsWith('prerelease=')) {
    const tag = type.split('=')[1];
    return `${major}.${minor}.${patch}-${tag}.0`;
  }

  if (type === 'prerelease') {
    if (!pre) {
      throw new Error(`Current version does not have a prerelease: ${current}`);
    }

    const preMatch = pre.match(/^([\w]+)\.(\d+)$/);
    if (!preMatch) {
      throw new Error(`Unsupported prerelease format: ${pre}`);
    }

    const [_, tag, num] = preMatch;
    return `${major}.${minor}.${patch}-${tag}.${+num + 1}`;
  }

  switch (type) {
    case 'major':
    case '1':
      return `${major + 1}.0.0`;
    case 'minor':
    case '2':
      return `${major}.${minor + 1}.0`;
    case 'patch':
    case '3':
      return `${major}.${minor}.${patch + 1}`;
    default:
      throw new Error(`Invalid type "${type}". Use major, minor, patch, prerelease or prerelease=tag.`);
  }
}

/**
 * Updates the version string in all target files.
 * @param {string} version - The new version to write.
 */
function updateFiles(version) {
  updateJsonVersion(paths.packageJson, version);
  updateCargoToml(paths.cargoToml, version);
  updateIssueTemplate(paths.issueTemplate, version);
}

/**
 * Updates the `version` field in a package.json file.
 * @param {string} filePath - Path to the JSON file.
 * @param {string} version - New version string.
 */
function updateJsonVersion(filePath, version) {
  const json = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  json.version = version;
  fs.writeFileSync(filePath, `${JSON.stringify(json, null, 2)}\n`);
}

/**
 * Updates the version inside a Cargo.toml file.
 * @param {string} filePath - Path to Cargo.toml.
 * @param {string} version - New version string.
 */
function updateCargoToml(filePath, version) {
  const content = fs.readFileSync(filePath, 'utf8');
  const replaced = content.replace(
    /\[workspace\.package\]\nversion = ".*?"/,
    `[workspace.package]\nversion = "${version}"`,
  );
  fs.writeFileSync(filePath, replaced);
}

/**
 * Adds the new version to the version list in the bug-report.yaml issue template.
 * @param {string} filePath - Path to the YAML file.
 * @param {string} version - New version string.
 */
function updateIssueTemplate(filePath, version) {
  const content = fs.readFileSync(filePath, 'utf8');
  const match = content.match(/options:\n((\s+- .*\n)+)/);
  if (!match) {
    throw new Error('Cannot find options list in issue template.');
  }

  const currentVersions = match[1]
    .split('\n')
    .map((v) => v.trim().slice(2))
    .filter(Boolean);
  if (!currentVersions.includes(version)) {
    const updated = content.replace(match[0], `${match[0]}        - ${version}\n`);
    fs.writeFileSync(filePath, updated);
  }
}

/**
 * Commits changes and creates a Git tag for the new version.
 * @param {string} oldVer - Previous version.
 * @param {string} newVer - New version.
 */
function commitAndTag(oldVer, newVer) {
  const tagArgs = useGpg ? '-s' : '';
  const commitArgs = useGpg ? '-S' : '';

  try {
    execSync('git add .');
    execSync(`git commit ${commitArgs} -m "build(version): bump from ${oldVer} to ${newVer}"`);
    execSync(`git tag ${tagArgs} ${newVer} -m "v${newVer}"`);
    log(`Git commit and tag created for v${newVer}`);
  } catch (e) {
    throw new Error(`Git operation failed: ${e.message}`);
  }
}

/**
 * Logs a message with script prefix.
 * @param {string} msg - The message to log.
 */
function log(msg) {
  console.log(`[version_up] ${msg}`);
}
