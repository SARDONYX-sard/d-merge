import FileOpen from '@mui/icons-material/FileOpen';
import { useState } from 'react';
import { useTranslation } from '@/components/hooks/useTranslation';
import { BackupButton } from '@/components/organisms/BackupButton';
import { NOTIFY } from '@/lib/notify';
import { type Cache, STORAGE } from '@/lib/storage';
import { BACKUP, ParserMode } from '@/services/api/backup';

import type { DialogClickHandler } from '@/components/organisms/BackupMenuDialog';

type Props = {
  parserMode: ParserMode;
};

export const BackupImportButton = ({ parserMode }: Props) => {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<Cache>({});
  const [open, setOpen] = useState(false);

  const handleClick = () => {
    void NOTIFY.asyncTry(async () => {
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

  const namePrefix = ((parserMode: ParserMode) => {
    switch (parserMode) {
      case 'egui':
        return 'egui ';
      case 'egui_v2':
        return 'egui v2 ';
      case 'tauri':
      default:
        return 'tauri';
    }
  })(parserMode);

  return (
    <BackupButton
      buttonName={`${namePrefix}${t('backup.import.button_name')}`}
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
