import { Box, Button } from '@mui/material';
import { useTranslation } from '../../../hooks/useTranslation';
import { useHkannoCommands } from '../hooks/useHkannoCommands';
import { ClosableTabs } from './ClosableTabs';

export const TopBar = () => {
  const { handleOpenClick } = useHkannoCommands();
  const { t } = useTranslation();

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        px: 1,
        borderBottom: '1px solid #333',
        bgcolor: '#1e1e1e',
      }}
    >
      <ClosableTabs />

      <Box sx={{ flexGrow: 1 }} />
      <Button variant='outlined' color='primary' size='small' onClick={handleOpenClick}>
        {t('select_button')}
      </Button>
    </Box>
  );
};
