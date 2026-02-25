import FileDownloadIcon from '@mui/icons-material/FileDownload';
import { useState } from 'react';

import { useTranslation } from '@/components/hooks/useTranslation';
import { BackupButton } from '@/components/organisms/BackupButton';
import type { DialogClickHandler } from '@/components/organisms/BackupMenuDialog';
import { NOTIFY } from '@/lib/notify';
import { STORAGE } from '@/lib/storage';
import { BACKUP } from '@/services/api/backup';
import { TAURI_KEYS_USED_BY_EGUI } from '@/services/api/backup/egui_support';

type Props = {
  parserMode: 'egui' | 'tauri';
};

export const BackupExportButton = ({ parserMode }: Props) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  const handleClick: DialogClickHandler = (checkedKeys) => {
    NOTIFY.asyncTry(async () => {
      if (await BACKUP.export(STORAGE.getByKeys(checkedKeys), parserMode)) {
        NOTIFY.success(t('backup.export.success_message'));
        setOpen(false);
      }
    });
  };

  const namePostfix = parserMode === 'egui' ? '(egui)' : '';

  return (
    <BackupButton
      buttonName={`${namePostfix}${t('backup.export.button_name')}`}
      cacheItems={STORAGE.getByKeys(TAURI_KEYS_USED_BY_EGUI)}
      inDialogClick={handleClick}
      onClick={() => setOpen(true)}
      open={open}
      setOpen={setOpen}
      startIcon={<FileDownloadIcon />}
      title={t('backup.export.dialog_title')}
      tooltipTitle={t('backup.export.tooltip')}
    />
  );
};
