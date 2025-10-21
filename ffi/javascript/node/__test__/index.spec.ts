import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { behaviorGen, Config, loggerInit, OutPutTarget, PatchStatus } from 'd_merge_node';
import { describe, it } from 'vitest';

describe('behavior_gen', () => {
  it('executes successfully', async () => {
    loggerInit(path.join(__dirname, 'logs'), 'node_ffi_test.log');

    const config = {
      resourceDir: path.resolve(__dirname, '../../../../resource/assets/templates'),
      outputDir: path.resolve(__dirname, '../__test__/out'),
      outputTarget: OutPutTarget.SkyrimSE,
      debug: {
        outputPatchJson: false,
        outputMergedJson: false,
        outputMergedXml: false,
      },
      hackOptions: {
        castRagdollEvent: true,
      },
      skyrimDataDirGlob: '../../../../dummy/fnis_test_mods',
    } as const satisfies Config;

    const nemesisEntries = await (async () => {
      const idsPath = path.resolve(__dirname, '../../../../dummy/ids.ini');
      const lines = (await readFile(idsPath, 'utf-8'))
        .split(/\r?\n/)
        // Skip empty lines and those starting with ";"
        .filter((line) => line.trim() !== '' && !line.trim().startsWith(';'))
        // Remove inline comments as well (e.g. "id1 ; comment" → "id1")
        .map((line) => line.split(';')[0].trim());

      return Object.fromEntries(lines.map((key, index) => [key, index]));
    })();
    const entries = {
      nemesisEntries,
      fnisEntries: {},
    };

    try {
      await behaviorGen(entries, config, newStatusLogger());
    } catch (e) {
      throw e; // test failed
    }
  });
});

function newStatusLogger() {
  let startTime: number | null = null;

  const CLEAR_LINE = '\r\x1b[2K';
  const RESET = '\x1b[0m';
  const CYAN = '\x1b[36m';
  const MAGENTA = '\x1b[35m';
  const YELLOW = '\x1b[33m';
  const BLUE = '\x1b[34m';
  const GREEN_BOLD = '\x1b[32;1m';
  const RED_BOLD = '\x1b[31;1m';

  const getColor = (type: PatchStatus['type']): string => {
    switch (type) {
      case 'GeneratingFnisPatches':
        return CYAN;
      case 'ReadingPatches':
        return BLUE;
      case 'ParsingPatches':
        return MAGENTA;
      case 'ApplyingPatches':
        return YELLOW;
      case 'GeneratingHkxFiles':
        return MAGENTA;
      case 'Done':
        return GREEN_BOLD;
      case 'Error':
        return RED_BOLD;
      default:
        return RESET;
    }
  };

  const formatStatus = (status: PatchStatus): string => {
    switch (status.type) {
      case 'GeneratingFnisPatches':
        return `[1/6] Generating FNIS patches... (${status.index}/${status.total})`;
      case 'ReadingPatches':
        return `[2/6] Reading templates and patches... (${status.index}/${status.total})`;
      case 'ParsingPatches':
        return `[3/6] Parsing patches... (${status.index}/${status.total})`;
      case 'ApplyingPatches':
        return `[4/6] Applying patches... (${status.index}/${status.total})`;
      case 'GeneratingHkxFiles':
        return `[5/6] Generating .hkx files... (${status.index}/${status.total})`;
      case 'Done':
        return `[6/6] ✅ behavior_gen executed successfully.`;
      case 'Error':
        return `[Error] ${status.field0}`;
    }
  };

  // This is the actual callback function
  return (_err: Error | null, status: PatchStatus) => {
    if (startTime === null) startTime = Date.now();

    const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
    const text = formatStatus(status);
    const color = getColor(status.type);

    process.stdout.write(`${CLEAR_LINE}${color}${elapsed}s: ${text}${RESET}`);
  };
}
