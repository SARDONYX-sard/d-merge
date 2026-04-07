import DoneIcon from '@mui/icons-material/Done';
import { Button, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, Tooltip } from '@mui/material';
import { ToolbarButton, useGridApiContext } from '@mui/x-data-grid';
import { useMemo, useState } from 'react';
import { useTranslation } from '@/components/hooks/useTranslation';
import { usePatchContext } from '@/components/providers/PatchProvider';

import type { ModItem } from '@/services/api/patch';

// FIXME: The issue where the internal valid state changes one step behind has not been identified, so it is currently unusable.
export const useSortApplyButton = () => {
  const { current: apiRefCurrent } = useGridApiContext();
  const { lockedDnd, isVfsMode, setModList, setVfsModList } = usePatchContext();
  const { t } = useTranslation();

  const [open, setOpen] = useState(false);

  const sortedRows = useMemo(() => {
    if (!apiRefCurrent) return [];
    return apiRefCurrent.getSortedRows();
  }, [apiRefCurrent]);

  const changedCount = useMemo(() => {
    return sortedRows.filter((row, index) => row.priority !== index).length;
  }, [sortedRows]);

  const handleConfirmApply = () => {
    if (!apiRefCurrent) return;

    const updatedList = sortedRows.map((row, index) => ({
      ...row,
      priority: index,
    })) as ModItem[];

    if (isVfsMode) {
      setVfsModList(updatedList);
    } else {
      setModList(updatedList);
    }

    apiRefCurrent.setSortModel([]);
    setOpen(false);
  };

  if (!lockedDnd) {
    return null;
  }

  return (
    <>
      <Tooltip title={t('patch.toolbar.apply_current_sort_order')}>
        <ToolbarButton aria-label='Apply current sort order' onClick={() => setOpen(true)}>
          <DoneIcon fontSize='small' />
        </ToolbarButton>
      </Tooltip>

      <Dialog open={open} onClose={() => setOpen(false)}>
        <DialogTitle>{t('patch.toolbar.confirm_apply_title')}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {changedCount === 0
              ? t('patch.toolbar.no_changes_detected')
              : `${changedCount} ${t('patch.toolbar.confirm_apply_message')}`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setOpen(false)}>{t('general.cancel_button')}</Button>
          <Button onClick={handleConfirmApply} color='warning' variant='contained' disabled={changedCount === 0}>
            {t('general.apply_button')}
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
};
