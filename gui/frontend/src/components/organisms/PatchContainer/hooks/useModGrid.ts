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

const reorderAndReindex = <T extends { priority: number }>(array: T[], oldIndex: number, newIndex: number): T[] => {
  return arrayMove(array, oldIndex, newIndex).map((item, idx) => ({
    ...item,
    priority: idx + 1,
  }));
};

type DragEndHandler = Exclude<DndContextProps['onDragEnd'], undefined>;
type OnRowChange = Exclude<DataGridPropsWithoutDefaultValue['onRowSelectionModelChange'], undefined>;

export const useModsGrid = () => {
  const { modInfoList, setModInfoList, lockedDnd } = usePatchContext();
  const { loading } = useFetchModInfo();
  const columns = useColumns();
  const apiRef = useGridApiRef();

  const handleDragEnd = useCallback<DragEndHandler>(
    ({ active, over }) => {
      if (over) {
        const oldIndex = modInfoList.findIndex((row) => row.id === active.id);
        const newIndex = modInfoList.findIndex((row) => row.id === over.id);
        setModInfoList((prevRows) => reorderAndReindex(prevRows, oldIndex, newIndex));
      }
    },
    [modInfoList, setModInfoList],
  );

  const handleRowSelectionModelChange = useCallback<OnRowChange>(
    (RowId, _detail) => {
      // NOTE: When the value is less than or equal to 0, there is no data and the selection is all cleared during data dir input.
      // To prevent this, skip judgment is performed.
      if (modInfoList.length <= 0) {
        return;
      }

      // HACK: For some reason, the check status becomes apparent one turn after checking, so it forces a “check all” at the zero stage.
      if (selectedIds.size === 0 && _detail.reason === 'multipleRowsSelection') {
        setModInfoList((prevModList: ModInfo[]) => {
          return prevModList.map((mod) => ({
            ...mod,
            enabled: true,
          }));
        });

        return;
      }

      setModInfoList((prevModList: ModInfo[]) => {
        return prevModList.map((mod) => ({
          ...mod,
          enabled: RowId.ids.has(mod.id),
        }));
      });
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
