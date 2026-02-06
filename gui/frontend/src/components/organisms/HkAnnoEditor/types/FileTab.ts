import z from 'zod';
import { HkannoSchema } from '@/services/api/hkanno';

/** Editor tab state */
export const FileTabSchema = z.object({
  id: z.string(),
  inputPath: z.string(),
  outputPath: z.string(),
  format: z.enum(['amd64', 'win32', 'xml', 'json']),
  ptr: z.string(),
  num_original_frames: z.number(),
  duration: z.number(),
  /** hkanno raw string */
  text: z.string(),
  /** struct Hkanno from rust backend */
  hkanno: HkannoSchema.readonly(),
  dirty: z.boolean().optional(),
  cursorPos: z
    .object({
      lineNumber: z.number(),
      column: z.number(),
    })
    .optional(),
});

export type FileTab = z.infer<typeof FileTabSchema>;
