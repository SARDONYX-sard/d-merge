import { DndContextProps } from '@dnd-kit/core';
import { arrayMove } from '@dnd-kit/sortable';
import { useGridApiRef } from '@mui/x-data-grid';
import { DataGridPropsWithoutDefaultValue } from '@mui/x-data-grid/internals';
import { useCallback, useMemo } from 'react';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import type { ModItem } from '@/services/api/patch';
import { useColumns } from './useColumns';
import { useFetchModInfo } from './useFetchModInfo';
import { useGridStatePersistence } from './useGridStatePersistence';

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
  const { loading } = useFetchModInfo();
  const columns = useColumns();
  const apiRef = useGridApiRef();

  // NOTE: Due to a design flaw, obtaining the prev value from the function arguments causes a bug.
  // Therefore, always overwrite it using `modInfoList`.
  const handleDragEnd = useCallback<DragEndHandler>(
    ({ active, over }) => {
      if (over) {
        const modInfoList = isVfsMode ? vfsModList : modList;

        const oldIndex = modInfoList.findIndex((row) => row.id === active.id);
        const newIndex = modInfoList.findIndex((row) => row.id === over.id);
        const newModInfoList = reorderAndReindex(modInfoList, oldIndex, newIndex);
        if (isVfsMode) {
          setVfsModList(newModInfoList);
        } else {
          setModList(newModInfoList);
        }
      }
    },
    [isVfsMode, vfsModList, setVfsModList, modList, setModList],
  );

  const handleRowSelectionModelChange = useCallback<OnRowChange>(
    (RowId, _detail) => {
      const modInfoList = isVfsMode ? vfsModList : modList;
      const setModInfoList = isVfsMode ? setVfsModList : setModList;

      // HACK: For some reason, the check status becomes apparent one turn after checking, so it forces a “check all” at the zero stage.
      if (selectedIds.size === 0 && _detail.reason === 'multipleRowsSelection') {
        setModInfoList(
          modInfoList.map((mod) => ({
            ...mod,
            enabled: true,
          })),
        );
        return;
      }

      setModInfoList(
        modInfoList.map((mod) => ({
          ...mod,
          enabled: RowId.ids.has(mod.id),
        })),
      );
    },
    [isVfsMode, vfsModList, setVfsModList, modList, setModList],
  );

  const selectedIds = useMemo(
    () => new Set((isVfsMode ? vfsModList : modList).filter((mod) => mod.enabled).map((mod) => mod.id)),
    [isVfsMode, vfsModList, modList],
  );

  useGridStatePersistence(apiRef, PUB_CACHE_OBJ.modsGridState);

  return {
    apiRef,
    columns,
    loading,
    handleDragEnd,
    handleRowSelectionModelChange,
    selectedIds,
    lockedDnd,
  };
};
