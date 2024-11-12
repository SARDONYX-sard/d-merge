import { z } from 'zod';

export const snackbarOriginSchema = z
  .object({
    vertical: z.enum(['top', 'bottom']).catch('top'),
    horizontal: z.enum(['left', 'center', 'right']).catch('left'),
  })
  .catch({
    horizontal: 'left',
    vertical: 'top',
  });

export const snackbarLimitSchema = z.number().int().positive().catch(3);
