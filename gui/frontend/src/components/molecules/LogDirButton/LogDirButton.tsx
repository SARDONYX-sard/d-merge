import FolderOpenIcon from '@mui/icons-material/FolderOpen';

import { ButtonWithToolTip } from '@/components/atoms/ButtonWithToolTip';
import { useTranslation } from '@/components/hooks/useTranslation';
import { NOTIFY } from '@/lib/notify';
import { LOG } from '@/services/api/log';

import type { ButtonProps } from '@mui/material';

type Props = ButtonProps;

export const LogDirButton = ({ ...props }: Props) => {
  const { t } = useTranslation();
  const handleClick = () => NOTIFY.asyncTry(async () => await LOG.openDir());

  return (
    <ButtonWithToolTip
      {...props}
      buttonName={t('open-log-dir-btn')}
      onClick={handleClick}
      startIcon={<FolderOpenIcon />}
      // tooltipTitle={t('open-log-dir-tooltip')}
    />
  );
};
