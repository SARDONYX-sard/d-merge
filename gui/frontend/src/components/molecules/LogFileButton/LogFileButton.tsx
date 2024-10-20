import FileOpen from '@mui/icons-material/FileOpen';

import { ButtonWithToolTip } from '@/components/atoms/ButtonWithToolTip';
import { useTranslation } from '@/components/hooks/useTranslation';
import { NOTIFY } from '@/lib/notify';
import { LOG } from '@/services/api/log';

export const LogFileButton = () => {
  const { t } = useTranslation();
  const handleClick = () => NOTIFY.asyncTry(async () => await LOG.openFile());

  return (
    <ButtonWithToolTip
      buttonName={t('open-log-btn')}
      onClick={handleClick}
      startIcon={<FileOpen />}
      // tooltipTitle={t('open-log-tooltip')}
    />
  );
};
