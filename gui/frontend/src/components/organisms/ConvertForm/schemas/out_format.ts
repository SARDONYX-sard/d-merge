import { z } from 'zod';

import type { OutFormat } from '@/services/api/serde_hkx';

import { schemaForType } from '.';

export const outFormatSchema = schemaForType<OutFormat>()(
  // NOTE: Do not use yaml because it cannot be reversed.
  // z.enum(['amd64', 'win32', 'xml', 'json', 'yaml']).catch('amd64'),
  z
    .enum(['amd64', 'win32', 'xml', 'json'])
    .catch('amd64'),
);
