//@ts-check
/**
 *
 * The purpose of this is to create the following directory structure:
 *
 * /dist/
 * |______ d_merge/*exe
 * |______ interface/.keep
 */

const fs = require('fs');
const path = require('path');
const archiver = require('archiver');
const { Arch } = require('app-builder-lib');

/**
 * Get the Rust target like platform name based on the Electron platform name.
 *
 * https://www.electron.build/app-builder-lib.typealias.electronplatformname
 *
 * # Error
 * If the `electronPlatformName` is not recognized, an error is thrown.
 */
const getPlatFormName = (/** @type {string} */ electronPlatformName) => {
  switch (electronPlatformName) {
    case 'win32':
      return 'Windows';
    case 'darwin':
      return 'macOS';
    case 'linux':
      return 'Linux';
    case 'mas':
      return 'Mas';
    default:
      throw new Error(`Unknown electronPlatformName: ${electronPlatformName}`);
      break;
  }
};

/**
 * @param {import("app-builder-lib").AfterPackContext} context
 */
module.exports = async (context) => {
  const { appOutDir, packager, arch, outDir, electronPlatformName } = context;

  const product = packager.appInfo.productFilename; // usually "d_merge"
  const version = packager.appInfo.version;
  const archName = Arch[arch];
  const platformName = getPlatFormName(electronPlatformName);

  const outZip = path.join(outDir, `${product}_electron-${archName}-${platformName}-${version}.zip`);

  await new Promise((resolve, reject) => {
    const output = fs.createWriteStream(outZip);
    const zip = archiver('zip', { zlib: { level: 9 } });

    output.on('close', () => resolve(null));
    zip.on('error', reject);
    zip.pipe(output);

    // 1) Put the whole app into d_merge/
    zip.directory(appOutDir + '/', 'd_merge');

    // 2) Add interface/.keep (create the file if missing)
    const keepDir = path.resolve(__dirname, '../dist/interface');
    const keepFile = path.join(keepDir, '.keep');
    if (!fs.existsSync(keepDir)) {
      fs.mkdirSync(keepDir, { recursive: true });
    }
    if (!fs.existsSync(keepFile)) {
      fs.writeFileSync(keepFile, '');
    }
    zip.file(keepFile, { name: 'interface/.keep' });

    zip.finalize();
  });

  // Notify electron-builder about this artifact
  packager.info.emitArtifactBuildCompleted({
    file: outZip,
    safeArtifactName: path.basename(outZip),
    target: null,
    arch,
    packager,
    isWriteUpdateInfo: false,
  });
};
