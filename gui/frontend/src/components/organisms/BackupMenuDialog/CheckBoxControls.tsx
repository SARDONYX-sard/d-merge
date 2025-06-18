import { Box, Checkbox, FormControlLabel } from '@mui/material';
import type { MouseEventHandler } from 'react';
import { useTranslation } from '@/components/hooks/useTranslation';

type Props = {
  isAllChecked: boolean;
  isPubAllChecked: boolean;
  onAllCheck: MouseEventHandler<HTMLButtonElement>;
  onPubCheck: MouseEventHandler<HTMLButtonElement>;
};

export const CheckBoxControls = ({ isAllChecked, isPubAllChecked, onAllCheck, onPubCheck }: Props) => {
  const { t } = useTranslation();
  return (
    <Box>
      <FormControlLabel
        checked={isAllChecked}
        control={<Checkbox onClick={onAllCheck} />}
        label={t('backup.dialog.all_checked_label')}
      />
      <FormControlLabel
        checked={isPubAllChecked}
        control={<Checkbox onClick={onPubCheck} />}
        label={t('backup.dialog.public_checked_label')}
      />
    </Box>
  );
};
