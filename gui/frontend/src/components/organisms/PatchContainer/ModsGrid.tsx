import { arrayMove } from '@dnd-kit/sortable';
import { useCallback } from 'react';

import { DraggableDataGrid } from '@/components/molecules/DraggableGrid/DraggableDataGrid';

import { usePatchContext } from './PatchProvider';
import { useColumns } from './useColumns';

import type { Props as DndCtxProps } from '@dnd-kit/core/dist/components/DndContext/DndContext';
import type { DataGridPropsWithoutDefaultValue } from '@mui/x-data-grid/internals';
import type { ComponentPropsWithRef } from 'react';

type DragEndHandler = Exclude<DndCtxProps['onDragEnd'], undefined>;

type Props = Partial<ComponentPropsWithRef<typeof DraggableDataGrid>>;

export function ModsGrid({ ...props }: Props) {
  const { rows, setRows, selectionModel, setSelectionModel } = usePatchContext();
  const columns = useColumns();

  const handleDragEnd = useCallback<DragEndHandler>(
    ({ active, over }) => {
      if (over) {
        const oldIndex = rows.findIndex((row) => row.id === active.id);
        const newIndex = rows.findIndex((row) => row.id === over.id);
        setRows((prevRows) => arrayMove(prevRows, oldIndex, newIndex));
      }
    },
    [rows, setRows],
  );

  const handleRowSelectionModelChange: DataGridPropsWithoutDefaultValue['onRowSelectionModelChange'] = (RowId) => {
    const selectedRowId = new Set(RowId);
    const selectedIds: string[] = [];

    for (const row of rows) {
      if (selectedRowId.has(row.id)) {
        selectedIds.push(row.id);
      }
    }
    setSelectionModel(selectedIds);
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
      onDragEnd={handleDragEnd}
      onRowSelectionModelChange={handleRowSelectionModelChange}
      rowSelectionModel={selectionModel}
      rows={rows}
      {...props}
    />
  );
}
