import { Box } from '@mui/material';
import { useTranslation } from '@/components/hooks/useTranslation';

export const EmptyDragPoint = () => {
  const { t } = useTranslation();

  return (
    <Box
      sx={{
        flexGrow: 1,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        color: '#777',
      }}
    >
      {t('hkanno.editor.dropping_message')}
    </Box>
  );
};
