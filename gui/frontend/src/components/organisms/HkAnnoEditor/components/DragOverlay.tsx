import { Box } from '@mui/material';
import { useTranslation } from '@/components/hooks/useTranslation';

export const DragOverlay = () => {
  const { t } = useTranslation();

  return (
    <Box
      sx={{
        position: 'absolute',
        inset: 0,
        backgroundColor: 'rgba(66,165,245,0.15)',
        border: '3px dashed #42a5f5',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        color: '#42a5f5',
        fontSize: '1.5rem',
        fontWeight: 500,
        zIndex: 1000,
      }}
    >
      {t('hkanno.editor.drop_instruction')}
    </Box>
  );
};
