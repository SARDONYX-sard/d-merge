import { invoke } from '@tauri-apps/api/core';
import { OutFormat } from './serde_hkx';

/** hkStringPtr/hkCString XML null display */
export const NULL_STR = '\u2400';

export type Annotation = {
  time: number;
  text: string | null;
};

export type AnnotationTrack = {
  annotations: Annotation[];
};

export type Hkanno = {
  /** XML index e.g. `#0003`  */
  ptr: string;
  num_original_frames: number;
  duration: number;
  annotation_tracks: AnnotationTrack[];
};

/**
 * Loads a .hkx or .xml file and parses it as an Hkanno structure.
 *
 * @throws If failed to load hkanno.
 */
export async function loadHkanno(path: string): Promise<Hkanno> {
  try {
    const result = await invoke<Hkanno>('load_hkanno', { input: path });
    return result;
  } catch (e) {
    throw e;
  }
}

/**
 * Saves updated Hkanno data back into an .hkx or .xml file.
 *
 * @param input   Original .hkx/.xml path
 * @param output  Output path to write updated file
 * @param format  Output format
 * @param hkanno  The modified Hkanno structure
 *
 * @throws If failed to save hkanno to file.
 */
export async function saveHkanno(input: string, output: string, format: OutFormat, hkanno: Hkanno): Promise<void> {
  try {
    await invoke('save_hkanno', {
      input,
      output,
      hkanno,
      format,
    });
  } catch (e) {
    throw e;
  }
}

/** Parse hkanno text into AnnotationTrack[] only */
export function hkannoFromText(text: string): AnnotationTrack[] {
  const lines = text.split('\n');
  const annotation_tracks: AnnotationTrack[] = [];
  let currentTrack: AnnotationTrack | null = null;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;

    if (trimmed.startsWith('# numAnnotations')) {
      if (currentTrack) {
        annotation_tracks.push(currentTrack);
      }
      currentTrack = { annotations: [] };
      continue;
    }

    if (trimmed.startsWith('#')) continue; // other comment lines

    if (!currentTrack) {
      // First track without numAnnotations header
      currentTrack = { annotations: [] };
    }

    const [t, ...txt] = trimmed.split(' ');
    const time = parseFloat(t);
    const annText = txt.join(' ');
    currentTrack.annotations.push({
      time,
      text: annText === NULL_STR ? null : annText,
    });
  }

  if (currentTrack) annotation_tracks.push(currentTrack);

  return annotation_tracks;
}
