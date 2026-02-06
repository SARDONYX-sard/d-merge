import { Box, Button } from '@mui/material';
import { useTranslation } from '@/components/hooks/useTranslation';
import { useEditorContext } from '../context/editorContext';
import { useHkannoCommands } from '../hooks/useHkannoCommands';

/** Top toolbar with save and preview controls */
export const HeaderToolbar = () => {
  const { t } = useTranslation();
  const [state, dispatch] = useEditorContext();
  const { saveCurrent } = useHkannoCommands();
  const hasTab = Boolean(state.tabs[state.active]);

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 1,
        px: 1,
        py: 0.5,
        borderBottom: '1px solid #333',
        bgcolor: '#1e1e1e',
      }}
    >
      <Button variant='contained' size='small' disabled={!hasTab} onClick={() => saveCurrent()}>
        {t('hkanno.toolbar.save')}
      </Button>

      <Button
        variant='outlined'
        size='small'
        disabled={!hasTab}
        onClick={() => dispatch({ type: 'REVERT_ACTIVE_TAB' })}
      >
        {t('hkanno.toolbar.revert')}
      </Button>

      <Box sx={{ flexGrow: 1 }} />

      <Button variant='text' size='small' onClick={() => dispatch({ type: 'TOGGLE_PREVIEW' })}>
        {state.showPreview ? t('hkanno.toolbar.hide_preview') : t('hkanno.toolbar.show_preview')}
      </Button>
    </Box>
  );
};
