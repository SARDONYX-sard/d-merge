import FolderOpenIcon from '@mui/icons-material/FolderOpen';
import { Button as Button_, type ButtonProps, type SxProps, type Theme } from '@mui/material';

import { useTranslation } from '@/components/hooks/useTranslation';

type Props = ButtonProps;

const defaultStyle = {
  marginTop: '9px',
  width: '150px',
  height: '55px',
} as const satisfies SxProps<Theme>;

export function Button({ sx, ...props }: Props) {
  const { t } = useTranslation();

  return (
    <Button_ startIcon={<FolderOpenIcon />} sx={{ ...defaultStyle, ...sx }} type='button' variant='outlined' {...props}>
      {t('select_button')}
    </Button_>
  );
}
