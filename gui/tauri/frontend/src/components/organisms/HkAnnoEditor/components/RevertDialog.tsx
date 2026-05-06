import { Dialog, DialogTitle, DialogContent, DialogActions, Button } from '@mui/material';
import { t } from 'i18next';
import { useCallback, useState, type MouseEventHandler } from 'react';
import { useEditorModeContext } from '../../../providers/EditorModeProvider';
import { MonacoEditor } from '../../MonacoEditor';
import { useEditorContext } from '../context/editorContext';
import { MONACO_OPTIONS } from './AnnotationEditor';
import { useTranslation } from '@/components/hooks/useTranslation';

export const RevertButton = ({ hasTab, originalText }: { hasTab: boolean; originalText?: string }) => {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [_, dispatch] = useEditorContext();

  const handleRevertClick = useCallback(() => {
    setConfirmOpen(true);
  }, []);

  const handleConfirm = useCallback(() => {
    dispatch({ type: 'REVERT_ACTIVE_TAB' });
    setConfirmOpen(false);
  }, [dispatch]);

  const handleCancel = useCallback(() => {
    setConfirmOpen(false);
  }, []);

  return (
    <>
      <Button variant='outlined' size='small' disabled={!hasTab} onClick={handleRevertClick}>
        {t('hkanno.toolbar.revert')}
      </Button>

      <RevertDialog open={confirmOpen} text={originalText || ''} onApply={handleConfirm} onCancel={handleCancel} />
    </>
  );
};

type DialogProps = {
  open: boolean;
  text: string;
  onApply: MouseEventHandler<HTMLButtonElement>;
  onAppend?: MouseEventHandler<HTMLButtonElement>;
  onCancel: MouseEventHandler<HTMLButtonElement>;
};

const RevertDialog = ({ open, text, onApply, onCancel }: DialogProps) => {
  const { editorMode } = useEditorModeContext();
  const { t } = useTranslation();

  return (
    <Dialog open={open} fullWidth maxWidth='md' onClose={onCancel}>
      <DialogTitle>{t('hkanno.toolbar.revert')}</DialogTitle>

      <DialogContent sx={{ height: '80vh' }}>
        <MonacoEditor
          value={text}
          language='hkanno'
          height={'90%'}
          options={{
            ...MONACO_OPTIONS,
            readOnly: true,
          }}
          vimMode={editorMode === 'vim'}
        />
      </DialogContent>

      <DialogActions>
        <Button variant='contained' onClick={onApply}>
          {t('general.apply_button')}
        </Button>

        <Button onClick={onCancel}>{t('general.cancel_button')}</Button>
      </DialogActions>
    </Dialog>
  );
};
