import { z } from 'zod';

import { schemaForType } from '.';

export type SelectionType = 'files' | 'dir' | 'tree';
export const selectionTypeSchema = schemaForType<SelectionType>()(z.enum(['files', 'dir', 'tree']).catch('tree'));
