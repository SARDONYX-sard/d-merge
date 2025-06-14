import LockOpenIcon from '@mui/icons-material/LockOpen';
import Tooltip from '@mui/material/Tooltip';
import { ToolbarButton, useGridApiContext } from '@mui/x-data-grid';
import { useEffect, useState } from 'react';

import { useTranslation } from '@/components/hooks/useTranslation';

export const useSortClearButton = () => {
  const { current: apiRefCurrent } = useGridApiContext();
  const [isSorted, setIsSorted] = useState(false);
  const { t } = useTranslation();

  useEffect(() => {
    if (apiRefCurrent === null) {
      return;
    }

    const updateSortState = () => {
      const sortModel = apiRefCurrent.getSortModel();
      setIsSorted(sortModel.length > 0);
    };

    updateSortState(); // first
    const unsubscribe = apiRefCurrent.subscribeEvent('sortModelChange', updateSortState);
    return () => unsubscribe();
  }, [apiRefCurrent]);

  const handleClearSort = () => {
    apiRefCurrent?.setSortModel([]);
  };

  const SortClearButton = isSorted ? (
    <Tooltip title={t('patch.toolbar.locked_due_to_sorting_help')}>
      <ToolbarButton aria-label='Clear sorting to unlock' color='primary' onClick={handleClearSort}>
        <LockOpenIcon fontSize='small' />
      </ToolbarButton>
    </Tooltip>
  ) : null;

  return SortClearButton;
};
