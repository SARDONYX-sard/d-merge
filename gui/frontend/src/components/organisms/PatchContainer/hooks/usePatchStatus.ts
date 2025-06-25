import { useState } from 'react';

import { useTranslation } from '@/components/hooks/useTranslation';
import { NOTIFY } from '@/lib/notify';
import type { Status } from '@/services/api/patch_listener';

export const usePatchStatus = (stop: () => string, setLoading: (v: boolean) => void) => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<Status | null>(null);
  const [statusText, setStatusText] = useState('');

  const handleStatus = (nextStatus: Status, unlisten: (() => void) | null) => {
    // NOTE: Unfortunately, when attempting to display the `index` and `total` in real time,
    // it does not display in time and is not used at all.
    //
    // const { index, total } = nextStatus.content;

    if (nextStatus === status) {
      return;
    }
    setStatus(nextStatus);

    let nextText = '';
    switch (nextStatus.type) {
      case 'ReadingPatches':
        nextText = t('patch.patch_reading_message');
        break;
      case 'ParsingPatches':
        nextText = t('patch.patch_parsing_message');
        break;
      case 'ApplyingPatches':
        nextText = t('patch.patch_applying_message');
        break;
      case 'GeneratingHkxFiles':
        nextText = t('patch.patch_generating_message');
        break;
      case 'Done': {
        nextText = `${t('patch.patch_complete_message')} (${stop()})`;
        setLoading(false);
        unlisten?.();
        break;
      }
      case 'Error': {
        nextText = `${t('patch.patch_error_message')} (${stop()})`;
        setLoading(false);
        unlisten?.();
        NOTIFY.error(nextStatus.content);
        break;
      }
      default:
        break;
    }

    if (nextText && nextText !== statusText) {
      setStatusText(nextText);
    }
  };
  return { status, statusText, handleStatus };
};
