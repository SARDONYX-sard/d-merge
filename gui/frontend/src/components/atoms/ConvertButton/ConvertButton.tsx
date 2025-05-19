import ConvertIcon from '@mui/icons-material/Transform';
import Button, { type ButtonProps } from '@mui/material/Button';

import { useTranslation } from '@/components/hooks/useTranslation';

type Props = ButtonProps & {
  buttonText?: string;
  loadingText?: string;
};

/**
 *
 * Icon ref
 * - https://mui.com/material-ui/material-icons/
 */
export function ConvertButton({ loading, buttonText, loadingText, ...props }: Props) {
  const { t } = useTranslation();

  return (
    <Button
      endIcon={<ConvertIcon />}
      loading={loading}
      loadingPosition='end'
      sx={{
        height: '55px',
        minWidth: '40%',
      }}
      type='button'
      variant='contained'
      {...props}
    >
      <span>
        {loading ? (loadingText ?? t('convert.converting_button')) : (buttonText ?? t('convert.button.convert'))}
      </span>
    </Button>
  );
}
