import FileOpen from '@mui/icons-material/FileOpen';
import {} from '@mui/material';
import { useState } from 'react';

import { useTranslation } from '@/components/hooks/useTranslation';
import { BackupButton } from '@/components/organisms/BackupButton';
import type { DialogClickHandler } from '@/components/organisms/BackupMenuDialog';
import { NOTIFY } from '@/lib/notify';
import { type Cache, STORAGE } from '@/lib/storage';
import { BACKUP } from '@/services/api/backup';

type Props = {
  parserMode: 'egui' | 'tauri';
};

export const BackupImportButton = ({ parserMode }: Props) => {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<Cache>({});
  const [open, setOpen] = useState(false);

  const handleClick = () => {
    NOTIFY.asyncTry(async () => {
      const newSettings = await BACKUP.import(parserMode);
      if (newSettings) {
        setSettings(newSettings);
        setOpen(true);
      }
    });
  };

  const handleDialogClick: DialogClickHandler = (checkedKeys) => {
    for (const key of checkedKeys) {
      const value = settings[key];
      if (value) {
        STORAGE.set(key, value);
      }
    }

    window.location.reload(); // To enable
  };

  const namePostfix = parserMode === 'egui' ? '(egui)' : '';

  return (
    <BackupButton
      buttonName={`      ${t('backup.import.button_name')}${namePostfix}`}
      cacheItems={settings}
      inDialogClick={handleDialogClick}
      onClick={handleClick}
      open={open}
      setOpen={setOpen}
      startIcon={<FileOpen />}
      title={t('backup.import.dialog_title')}
      tooltipTitle={t('backup.import.tooltip')}
    />
  );
};
