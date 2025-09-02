import LockOpenIcon from '@mui/icons-material/LockOpen';
import Tooltip from '@mui/material/Tooltip';
import { ToolbarButton, useGridApiContext } from '@mui/x-data-grid';
import { useEffect } from 'react';

import { useTranslation } from '@/components/hooks/useTranslation';
import { usePatchContext } from '@/components/providers/PatchProvider';

/**
 * Returns a toolbar button to clear sorting.
 * While sorted, dragging is locked (lock = true)
 */
export const useSortClearButton = () => {
  const { current: apiRefCurrent } = useGridApiContext();
  const { lockedDnd, setLockedDnd } = usePatchContext();
  const { t } = useTranslation();

  useEffect(() => {
    if (!apiRefCurrent) return;

    const updateSortState = () => {
      const sortModel = apiRefCurrent.getSortModel();
      setLockedDnd(sortModel.length > 0);
    };

    updateSortState(); // initialize
    const unsubscribe = apiRefCurrent.subscribeEvent('sortModelChange', updateSortState);
    return () => unsubscribe();
  }, [apiRefCurrent]);

  const handleClearSort = () => {
    apiRefCurrent?.setSortModel([]);
  };

  const SortClearButton = lockedDnd ? (
    <Tooltip title={t('patch.toolbar.locked_due_to_sorting_help')}>
      <ToolbarButton aria-label='Clear sorting to unlock' color='primary' onClick={handleClearSort}>
        <LockOpenIcon fontSize='small' />
      </ToolbarButton>
    </Tooltip>
  ) : null;

  return SortClearButton;
};
