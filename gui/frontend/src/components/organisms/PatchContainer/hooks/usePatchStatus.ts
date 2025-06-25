import { useState } from 'react';

import { useTranslation } from '@/components/hooks/useTranslation';
import { NOTIFY } from '@/lib/notify';
import type { Status } from '@/services/api/patch_listener';

export const usePatchStatus = (stop: () => string, setLoading: (v: boolean) => void) => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<Status | null>(null);
  const [statusText, setStatusText] = useState('');

  const handleStatus = (status: Status, unlisten: (() => void) | null) => {
    setStatus(status);

    switch (status.type) {
      case 'ReadingTemplatesAndPatches':
        let { index, total } = status.content;
        setStatusText(`${t('patch.patch_reading_message')} (${index}/${total})`);
        break;
      case 'ParsingPatches': {
        const { index, total } = status.content;
        setStatusText(`${t('patch.patch_parsing_message')} (${index}/${total})`);
        break;
      }
      case 'ApplyingPatches': {
        const { index, total } = status.content;
        setStatusText(`${t('patch.patch_applying_message')} (${index}/${total})`);
        break;
      }
      case 'GenerateHkxFiles': {
        const { index, total } = status.content;
        setStatusText(`${t('patch.patch_generating_message')} (${index}/${total})`);
        break;
      }
      case 'Done': {
        setStatusText(`${t('patch.patch_complete_message')} (${stop()})`);
        setLoading(false);
        unlisten?.();
        break;
      }
      case 'Error': {
        setLoading(false);
        unlisten?.();
        setStatusText(`${t('patch.patch_error_message')} (${stop()})`);
        NOTIFY.error(status.content);
        break;
      }
      default:
        break;
    }
  };

  return { status, statusText, handleStatus };
};
