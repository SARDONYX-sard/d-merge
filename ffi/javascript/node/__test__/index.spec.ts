import { readFile } from 'node:fs/promises';
import path from 'node:path';
// import { behaviorGen, Config, loggerInit, OutPutTarget, PatchStatus } from '../index';
import { behaviorGen, Config, loggerInit, OutPutTarget, PatchStatus } from 'd_merge_node';
import { describe, it } from 'vitest';

let startTime: number | null = null;

type ReportFn = (err: Error | null, arg: PatchStatus) => void;

// allow unused
// @ts-ignore
const onStatus: ReportFn = (_err, status) => {
  if (startTime === null) startTime = Date.now();

  const elapsed = (Date.now() - startTime) / 1000;
  const elapsedStr = `${elapsed.toFixed(1)}s: `;

  const CYAN = '\x1b[36m';
  const MAGENTA = '\x1b[35m';
  const YELLOW = '\x1b[33m';
  const BLUE = '\x1b[34m';
  const GREEN_BOLD = '\x1b[32;1m';
  const RED_BOLD = '\x1b[31;1m';
  const RESET = '\x1b[0m';
  const CLEAR_LINE = '\r\x1b[2K';

  const text = String();
  const displayText = elapsedStr + text;

  switch (status.type) {
    case 'ReadingPatches':
      process.stdout.write(`${CLEAR_LINE}${CYAN}${displayText}${RESET}`);
      break;
    case 'ParsingPatches':
      process.stdout.write(`${CLEAR_LINE}${BLUE}${displayText}${RESET}`);
      break;
    case 'ApplyingPatches':
      process.stdout.write(`${CLEAR_LINE}${MAGENTA}${displayText}${RESET}`);
      break;
    case 'GeneratingHkxFiles':
      process.stdout.write(`${CLEAR_LINE}${YELLOW}${displayText}${RESET}`);
      break;
    case 'Done':
      process.stdout.write(`${CLEAR_LINE}${GREEN_BOLD}${displayText}${RESET}`);
      break;
    case 'Error':
      process.stdout.write(`${CLEAR_LINE}${RED_BOLD}${displayText}${RESET}`);
      break;
    default:
      break;
  }
};

describe('behavior_gen', () => {
  it('executes successfully', async () => {
    const logDir = path.join(__dirname, 'logs');
    loggerInit(logDir, 'node_ffi_test.log');

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
    } as const satisfies Config;

    // Nemesis patch ids
    const idsPath = path.resolve(__dirname, '../../../../dummy/ids.txt');
    const paths = (await readFile(idsPath, 'utf-8')).split(/\r?\n/);

    try {
      // await behaviorGen(paths, config, onStatus);
      await behaviorGen(paths, config);
      console.log('✅ behavior_gen executed successfully.');
    } catch (e) {
      throw e; // vitest failed
    }
  });
});
