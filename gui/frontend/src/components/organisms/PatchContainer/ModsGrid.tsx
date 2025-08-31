import type { Props as DndCtxProps } from '@dnd-kit/core/dist/components/DndContext/DndContext';
import { arrayMove } from '@dnd-kit/sortable';
import { useGridApiRef } from '@mui/x-data-grid';
import type { DataGridPropsWithoutDefaultValue } from '@mui/x-data-grid/internals';
import type { ComponentPropsWithRef, FC } from 'react';
import { memo, useCallback } from 'react';
import { DraggableDataGrid } from '@/components/molecules/DraggableGrid/DraggableDataGrid';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { ModInfo } from '@/services/api/patch';
import { CustomToolbar } from './GridToolbar';
import { useColumns } from './hooks/useColumns';
import { useGridStatePersistence } from './hooks/useGridStatePersistence';

type DragEndHandler = Exclude<DndCtxProps['onDragEnd'], undefined>;
type OnRowChange = Exclude<DataGridPropsWithoutDefaultValue['onRowSelectionModelChange'], undefined>;

type Props = Partial<ComponentPropsWithRef<typeof DraggableDataGrid>>;

export const ModsGrid: FC<Props> = memo(function ModsGrid({ ...props }) {
  const { modInfoList, setModInfoList, loading, lockedDnd } = usePatchContext();
  const columns = useColumns();

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

      const selectedRowId = RowId.ids;

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
          enabled: selectedRowId.has(mod.id),
        }));
      });
    },
    [modInfoList],
  );

  const apiRef = useGridApiRef();
  useGridStatePersistence(apiRef, PUB_CACHE_OBJ.modsGridState);

  const selectedIds = new Set(modInfoList.filter((mod) => mod.enabled).map((mod) => mod.id));
  return (
    <DraggableDataGrid
      apiRef={apiRef}
      columns={columns}
      initialState={{
        columns: {
          columnVisibilityModel: {
            id: false,
            auto: false,
          },
        },
      }}
      keepNonExistentRowsSelected={true}
      loading={loading}
      onDragEnd={handleDragEnd}
      onRowSelectionModelChange={handleRowSelectionModelChange}
      rowSelectionModel={{
        ids: selectedIds,
        type: 'include',
      }}
      draggable={!lockedDnd}
      rows={modInfoList}
      showToolbar={true}
      slots={{ toolbar: CustomToolbar }}
      {...props}
    />
  );
});

/**
 * Move the array and rearrange the priorities.
 * @returns A new array (original array is not modified)
 */
const reorderAndReindex = <T extends { id: string; priority: number }>(
  array: T[],
  oldIndex: number,
  newIndex: number,
): T[] => {
  const newArray = arrayMove(array, oldIndex, newIndex).map((item, idx) => ({
    ...item,
    priority: idx + 1, // 1 based
  }));
  return newArray;
};
