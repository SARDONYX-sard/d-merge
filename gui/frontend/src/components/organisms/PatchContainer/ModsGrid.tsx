import type { ComponentPropsWithRef, FC } from 'react';
import { memo } from 'react';
import { DraggableDataGrid } from '@/components/molecules/DraggableGrid/DraggableDataGrid';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { CustomToolbar } from './GridToolbar';
import { useModsGrid } from './hooks/useModGrid';
import { useColumns } from './hooks/useModGrid/useColumns';
import { useFetchModInfo } from './hooks/useModGrid/useFetchModInfo';

type Props = Partial<ComponentPropsWithRef<typeof DraggableDataGrid>>;

export const ModsGrid: FC<Props> = memo(function ModsGrid({ ...props }) {
  const { isVfsMode, modList, vfsModList, fetchIsEmpty } = usePatchContext();
  const { loading } = useFetchModInfo();
  const columns = useColumns();
  const { apiRef, handleDragEnd, handleRowSelectionModelChange, selectedIds, lockedDnd } = useModsGrid();

  const rows = (() => {
    // Apply dummy to preserve check state.
    if (fetchIsEmpty) {
      return [];
    }
    return isVfsMode ? vfsModList : modList;
  })();

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
      rows={rows}
      showToolbar={true}
      slots={{ toolbar: CustomToolbar }}
      {...props}
    />
  );
});
