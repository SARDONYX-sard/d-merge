import type { Config } from 'd_merge_node';

export type ModInfo = {
  id: string;
  name: string;
  author: string;
  site: string;
};

export type ModIds = string[];

/** must be same as `GuiOption` serde */
export type PatchOptions = {
  /** Delete the meshes in the output destination each time the patch is run. */
  autoRemoveMeshes: boolean;
  /** Report progress status +2s */
  useProgressReporter: boolean;
} & Config;

export type PatchArguments = { outputDir: string; ids: ModIds; options: PatchOptions };
