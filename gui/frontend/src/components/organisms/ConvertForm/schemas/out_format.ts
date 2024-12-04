import { z } from 'zod';

import type { OutFormat } from '@/services/api/serde_hkx';

import { schemaForType } from '.';

export const outFormatSchema = schemaForType<OutFormat>()(
  z.enum(['amd64', 'win32', 'xml', 'json', 'yaml']).catch('amd64'),
);
