import { DndContextProps } from '@dnd-kit/core';
import { arrayMove } from '@dnd-kit/sortable';
import { useGridApiRef } from '@mui/x-data-grid';
import { DataGridPropsWithoutDefaultValue } from '@mui/x-data-grid/internals';
import { useCallback, useMemo } from 'react';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { ModInfo } from '@/services/api/patch';
import { useColumns } from './useColumns';
import { useFetchModInfo } from './useFetchModInfo';
import { useGridStatePersistence } from './useGridStatePersistence';

const reorderAndReindex = (array: ModInfo[], oldIndex: number, newIndex: number): ModInfo[] => {
  return arrayMove(array, oldIndex, newIndex).map((item, idx) => {
    return {
      ...item,
      enabled: item.enabled,
      priority: idx + 1,
    };
  });
};

type DragEndHandler = Exclude<DndContextProps['onDragEnd'], undefined>;
type OnRowChange = Exclude<DataGridPropsWithoutDefaultValue['onRowSelectionModelChange'], undefined>;

export const useModsGrid = () => {
  const { modInfoList, setModInfoList, lockedDnd } = usePatchContext();
  const { loading } = useFetchModInfo();
  const columns = useColumns();
  const apiRef = useGridApiRef();

  // NOTE: Due to a design flaw, obtaining the prev value from the function arguments causes a bug.
  // Therefore, always overwrite it using `modInfoList`.
  const handleDragEnd = useCallback<DragEndHandler>(
    ({ active, over }) => {
      if (over) {
        const oldIndex = modInfoList.findIndex((row) => row.id === active.id);
        const newIndex = modInfoList.findIndex((row) => row.id === over.id);
        setModInfoList(reorderAndReindex(modInfoList, oldIndex, newIndex));
      }
    },
    [modInfoList, setModInfoList],
  );

  const handleRowSelectionModelChange = useCallback<OnRowChange>(
    (RowId, _detail) => {
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
    [modInfoList],
  );

  const selectedIds = useMemo(
    () => new Set(modInfoList.filter((mod) => mod.enabled).map((mod) => mod.id)),
    [modInfoList],
  );

  useGridStatePersistence(apiRef, PUB_CACHE_OBJ.modsGridState);

  return {
    apiRef,
    columns,
    loading,
    handleDragEnd,
    handleRowSelectionModelChange,
    selectedIds,
    modInfoList,
    lockedDnd,
  };
};
