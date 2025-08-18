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
  hackOptions: {
    castRagdollEvent: boolean;
  };
  debug: {
    outputPatchJson: boolean;
    outputMergedJson: boolean;
    outputMergedXml: boolean;
  };
  outputTarget: 'SkyrimSE' | 'SkyrimLE';
  /** Delete the meshes in the output destination each time the patch is run. */
  autoRemoveMeshes: boolean;
  /** Report progress status +2s */
  useProgressReporter: boolean;
};

export type PatchArguments = { output: string; ids: ModIds; options: PatchOptions };
