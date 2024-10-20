import type { useModsInfo } from '@/components/hooks/useModsInfo';
import { DraggableDataGrid } from '@/components/molecules/DraggableGrid/DraggableDataGrid';

import { useColumns } from './useColumns';

import type { ComponentPropsWithRef } from 'react';

type Props = Partial<ComponentPropsWithRef<typeof DraggableDataGrid>> &
  Omit<ReturnType<typeof useModsInfo>, 'selectedRows'>;

export function ModsGrid({ rows, selectionModel, handleDragEnd, handleRowSelectionModelChange, ...props }: Props) {
  const columns = useColumns();

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
