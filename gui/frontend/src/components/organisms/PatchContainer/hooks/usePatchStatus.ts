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
        setStatusText(t('patch.patch_reading_message'));
        break;
      case 'ApplyingPatches':
        setStatusText(t('patch.patch_applying_message'));
        break;
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
        NOTIFY.error(status.message);
        break;
      }
      default:
        break;
    }
  };

  return { status, statusText, handleStatus };
};
