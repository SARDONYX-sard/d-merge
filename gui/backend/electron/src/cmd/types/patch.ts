import { DebugOptions, HackOptions, OutPutTarget } from '../ffi';

export type ModInfo = {
  id: string;
  name: string;
  author: string;
  site: string;
  auto: string;
};

export type ModIds = readonly string[];

/** must be same as `GuiOption` serde */
export type PatchOptions = {
  hackOptions: HackOptions;
  debug: DebugOptions;
  outputTarget: OutPutTarget;
  /** Delete the meshes in the output destination each time the patch is run. */
  autoRemoveMeshes: boolean;
  /** Report progress status +2s */
  useProgressReporter: boolean;
};

export type PatchArguments = { outputDir: string; ids: ModIds; options: PatchOptions };
