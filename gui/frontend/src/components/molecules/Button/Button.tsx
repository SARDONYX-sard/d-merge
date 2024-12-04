import FolderOpenIcon from '@mui/icons-material/FolderOpen';
import { type ButtonProps, Button as Button_ } from '@mui/material';

import { useTranslation } from '@/components/hooks/useTranslation';

type Props = ButtonProps;

export function Button({ ...props }: Props) {
  const { t } = useTranslation();

  return (
    <Button_
      startIcon={<FolderOpenIcon />}
      sx={{
        marginTop: '9px',
        width: '150px',
        height: '55px',
      }}
      type='button'
      variant='outlined'
      {...props}
    >
      {t('select-btn')}
    </Button_>
  );
}
