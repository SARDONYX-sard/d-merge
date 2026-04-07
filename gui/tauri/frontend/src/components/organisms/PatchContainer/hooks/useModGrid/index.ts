import { DndContextProps } from '@dnd-kit/core';
import { arrayMove } from '@dnd-kit/sortable';
import { useGridApiRef } from '@mui/x-data-grid';
import { DataGridPropsWithoutDefaultValue } from '@mui/x-data-grid/internals';
import { useCallback, useMemo } from 'react';
import { useGridStatePersistence } from './useGridStatePersistence';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';

import type { ModItem } from '@/services/api/patch';

const reorderAndReindex = (array: ModItem[], oldIndex: number, newIndex: number): ModItem[] => {
  return arrayMove(array, oldIndex, newIndex).map((item, idx) => {
    return {
      ...item,
      enabled: item.enabled,
      priority: idx,
    };
  });
};

type DragEndHandler = Exclude<DndContextProps['onDragEnd'], undefined>;
type OnRowChange = Exclude<DataGridPropsWithoutDefaultValue['onRowSelectionModelChange'], undefined>;

export const useModsGrid = () => {
  const { isVfsMode, vfsModList, setVfsModList, modList, setModList, lockedDnd } = usePatchContext();
  const apiRef = useGridApiRef();

  // NOTE: Due to a design flaw, obtaining the prev value from the function arguments causes a bug.
  // Therefore, always overwrite it using `modInfoList`.
  const handleDragEnd = useCallback<DragEndHandler>(
    ({ active, over }) => {
      const setModInfoList = isVfsMode ? setVfsModList : setModList;
      setModInfoList((prev) => {
        const oldIndex = prev.findIndex((row) => row.id === active.id);
        const newIndex = prev.findIndex((row) => row.id === over?.id);

        if (oldIndex === -1 || newIndex === -1) return prev;

        return reorderAndReindex(prev, oldIndex, newIndex);
      });
    },
    [isVfsMode, vfsModList, setVfsModList, modList, setModList],
  );

  const handleRowSelectionModelChange = useCallback<OnRowChange>(
    (newSelectionModel, _detail) => {
      const setModInfoList = isVfsMode ? setVfsModList : setModList;

      // HACK: For some reason, the check status becomes apparent one turn after checking, so it forces a “check all” at the zero stage.
      if (selectedIds.size === 0 && _detail.reason === 'multipleRowsSelection') {
        setModInfoList((prevModList) => {
          return prevModList.map((mod) => ({
            ...mod,
            enabled: true,
          }));
        });

        return;
      }

      const newSelectedIds = new Set(newSelectionModel.ids);

      setModInfoList((prev) =>
        prev.map((mod) => ({
          ...mod,
          enabled: newSelectedIds.has(mod.id),
        })),
      );
    },
    [isVfsMode, setVfsModList, setModList],
  );

  const selectedIds = useMemo(
    () => new Set((isVfsMode ? vfsModList : modList).filter((mod) => mod.enabled).map((mod) => mod.id)),
    [isVfsMode, vfsModList, modList],
  );

  useGridStatePersistence(apiRef, PUB_CACHE_OBJ.modsGridState);

  return {
    apiRef,
    handleDragEnd,
    handleRowSelectionModelChange,
    selectedIds,
    lockedDnd,
  };
};
