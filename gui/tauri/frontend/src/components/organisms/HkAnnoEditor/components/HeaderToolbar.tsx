import { Box, Button } from '@mui/material';
import { useEditorContext } from '../context/editorContext';
import { useHkannoCommands } from '../hooks/useHkannoCommands';
import { LspOptionDialogButton } from './LspOptionDialogButton';
import { RevertButton } from './RevertDialog';
import { useTranslation } from '@/components/hooks/useTranslation';
import { hkannoToText } from '@/services/api/hkanno';

/** Top toolbar with save and preview controls */
export const HeaderToolbar = () => {
  const { t } = useTranslation();
  const [state, dispatch] = useEditorContext();
  const { saveCurrent } = useHkannoCommands();
  const tab = state.tabs[state.active];
  const hasTab = Boolean(tab);

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

      <RevertButton hasTab={hasTab} originalText={hkannoToText(tab.hkanno)} />

      <LspOptionDialogButton />

      <Box sx={{ flexGrow: 1 }} />

      <Button variant='text' size='small' onClick={() => dispatch({ type: 'TOGGLE_PREVIEW' })}>
        {state.showPreview ? t('hkanno.toolbar.hide_preview') : t('hkanno.toolbar.show_preview')}
      </Button>
    </Box>
  );
};
