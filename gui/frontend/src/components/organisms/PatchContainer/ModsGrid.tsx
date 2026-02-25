import type { ComponentPropsWithRef, FC } from 'react';
import { memo } from 'react';
import { DraggableDataGrid } from '@/components/molecules/DraggableGrid/DraggableDataGrid';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { CustomToolbar } from './GridToolbar';
import { useModsGrid } from './hooks/useModGrid';

type Props = Partial<ComponentPropsWithRef<typeof DraggableDataGrid>>;

export const ModsGrid: FC<Props> = memo(function ModsGrid({ ...props }) {
  const { isVfsMode, modList, vfsModList } = usePatchContext();
  const { apiRef, columns, loading, handleDragEnd, handleRowSelectionModelChange, selectedIds, lockedDnd } =
    useModsGrid();

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
      rows={isVfsMode ? vfsModList : modList}
      showToolbar={true}
      slots={{ toolbar: CustomToolbar }}
      {...props}
    />
  );
});
