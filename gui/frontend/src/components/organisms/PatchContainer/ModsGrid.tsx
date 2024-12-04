import { arrayMove } from '@dnd-kit/sortable';
import { useCallback } from 'react';

import { DraggableDataGrid } from '@/components/molecules/DraggableGrid/DraggableDataGrid';

import { usePatchContext } from './PatchProvider';
import { useColumns } from './useColumns';

import type { Props as DndCtxProps } from '@dnd-kit/core/dist/components/DndContext/DndContext';
import type { DataGridPropsWithoutDefaultValue } from '@mui/x-data-grid/internals';
import type { ComponentPropsWithRef, FC } from 'react';

type DragEndHandler = Exclude<DndCtxProps['onDragEnd'], undefined>;

type Props = Partial<ComponentPropsWithRef<typeof DraggableDataGrid>>;

export const ModsGrid: FC<Props> = ({ ...props }) => {
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

  const handleRowSelectionModelChange: DataGridPropsWithoutDefaultValue['onRowSelectionModelChange'] = (RowId) => {
    const selectedRowId = new Set(RowId);
    const selectedIds: string[] = [];

    for (const row of modInfoList) {
      if (selectedRowId.has(row.id)) {
        selectedIds.push(row.id);
      }
    }
    setActivateMods(selectedIds);
  };

  return (
    <DraggableDataGrid
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
      rowSelectionModel={activateMods}
      rows={modInfoList}
      {...props}
    />
  );
};
