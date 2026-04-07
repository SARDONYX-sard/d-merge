import { z } from 'zod';

export const boolSchema = z.boolean().catch(false);
export const stringArraySchema = z.array(z.string()).catch([]);
export const stringSchema = z.string().catch('');
