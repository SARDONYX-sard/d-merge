import { arrayMove } from '@dnd-kit/sortable';
import { useGridApiRef } from '@mui/x-data-grid';
import { memo, useCallback } from 'react';

import { DraggableDataGrid } from '@/components/molecules/DraggableGrid/DraggableDataGrid';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';

import { useColumns } from './hooks/useColumns';
import { useGridStatePersistence } from './hooks/useGridStatePersistence';

import type { Props as DndCtxProps } from '@dnd-kit/core/dist/components/DndContext/DndContext';
import type { DataGridPropsWithoutDefaultValue } from '@mui/x-data-grid/internals';
import type { ComponentPropsWithRef, FC } from 'react';

type DragEndHandler = Exclude<DndCtxProps['onDragEnd'], undefined>;
type OnRowChange = Exclude<DataGridPropsWithoutDefaultValue['onRowSelectionModelChange'], undefined>;

type Props = Partial<ComponentPropsWithRef<typeof DraggableDataGrid>>;

export const ModsGrid: FC<Props> = memo(function ModsGrid({ ...props }) {
  const { modInfoList, setModInfoList, activateMods, setActivateMods, loading, setPriorities } = usePatchContext();
  const columns = useColumns();

  const handleDragEnd = useCallback<DragEndHandler>(
    ({ active, over }) => {
      if (over) {
        const oldIndex = modInfoList.findIndex((row) => row.id === active.id);
        const newIndex = modInfoList.findIndex((row) => row.id === over.id);
        setModInfoList((prevRows) => {
          const newList = arrayMove(prevRows, oldIndex, newIndex);
          setPriorities(newList.map((row) => row.id));
          return newList;
        });
      }
    },
    [modInfoList, setModInfoList, setPriorities],
  );

  const handleRowSelectionModelChange = useCallback<OnRowChange>(
    (RowId) => {
      // NOTE: When the value is less than or equal to 0, there is no data and the selection is all cleared during data dir input.
      // To prevent this, skip judgment is performed.
      if (modInfoList.length <= 0) {
        return;
      }

      const selectedRowId = new Set(RowId.ids);
      const selectedIds: string[] = [];

      for (const row of modInfoList) {
        if (selectedRowId.has(row.id)) {
          selectedIds.push(row.id);
        }
      }
      setActivateMods(selectedIds);
    },
    [modInfoList, setActivateMods],
  );

  const apiRef = useGridApiRef();
  useGridStatePersistence(apiRef, PUB_CACHE_OBJ.modsGridState);

  return (
    <DraggableDataGrid
      apiRef={apiRef}
      columns={columns}
      density='compact'
      initialState={{
        columns: {
          columnVisibilityModel: {
            id: false,
            auto: false,
          },
        },
      }}
      loading={loading}
      onDragEnd={handleDragEnd}
      onRowSelectionModelChange={handleRowSelectionModelChange}
      rowSelectionModel={{
        ids: new Set(activateMods),
        type: 'include',
      }}
      rows={modInfoList}
      {...props}
    />
  );
});
