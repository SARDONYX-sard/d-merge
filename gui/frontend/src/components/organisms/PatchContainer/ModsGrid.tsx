import type { Props as DndCtxProps } from '@dnd-kit/core/dist/components/DndContext/DndContext';
import { arrayMove } from '@dnd-kit/sortable';
import { useGridApiRef } from '@mui/x-data-grid';
import type { DataGridPropsWithoutDefaultValue } from '@mui/x-data-grid/internals';
import type { ComponentPropsWithRef, FC } from 'react';
import { memo, useCallback } from 'react';
import { DraggableDataGrid } from '@/components/molecules/DraggableGrid/DraggableDataGrid';
import { ModItem, usePatchContext } from '@/components/providers/PatchProvider';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { ModInfo } from '../../../services/api/patch';
import { CustomToolbar } from './GridToolbar';
import { useColumns } from './hooks/useColumns';
import { useGridStatePersistence } from './hooks/useGridStatePersistence';

type DragEndHandler = Exclude<DndCtxProps['onDragEnd'], undefined>;
type OnRowChange = Exclude<DataGridPropsWithoutDefaultValue['onRowSelectionModelChange'], undefined>;

type Props = Partial<ComponentPropsWithRef<typeof DraggableDataGrid>>;

export const ModsGrid: FC<Props> = memo(function ModsGrid({ ...props }) {
  const {
    isVfsMode,

    modList,
    setModList,

    vfsModList,
    setVfsModList,

    modInfoList,
    setModInfoList,
    loading,
  } = usePatchContext();
  const columns = useColumns();

  const activeModList = isVfsMode ? vfsModList : modList;

  const handleDragEnd = useCallback<DragEndHandler>(
    ({ active, over }) => {
      if (over) {
        const oldIndex = modInfoList.findIndex((row) => row.id === active.id);
        const newIndex = modInfoList.findIndex((row) => row.id === over.id);

        setModInfoList((prevRows) => {
          // HACK: I don't know if it's a hidden specification, but drag-and-drop wouldn't work unless the drawn
          // information and the derived information matched perfectly.
          const newArray = reorderAndReindex(prevRows, oldIndex, newIndex);

          if (isVfsMode) {
            setVfsModList(toModList(newArray));
          } else {
            setModList(toModList(newArray));
          }

          return newArray;
        });
      }
    },
    [modInfoList, setModInfoList, setVfsModList, setModList],
  );

  const handleRowSelectionModelChange = useCallback<OnRowChange>(
    (RowId, _detail) => {
      // NOTE: When the value is less than or equal to 0, there is no data and the selection is all cleared during data dir input.
      // To prevent this, skip judgment is performed.
      if (modInfoList.length <= 0) {
        return;
      }

      const selectedRowId = new Set(RowId.ids);

      const modItemSetter = (prevModList: ModItem[]) => {
        return prevModList.map((mod) => ({
          ...mod,
          enabled: selectedRowId.has(mod.id),
        }));
      };

      if (isVfsMode) {
        setVfsModList(modItemSetter);
      } else {
        setModList(modItemSetter);
      }
    },
    [modInfoList, setVfsModList, setModList],
  );

  const apiRef = useGridApiRef();
  useGridStatePersistence(apiRef, PUB_CACHE_OBJ.modsGridState);

  const selectedIds = new Set(activeModList.filter((mod) => mod.enabled).map((mod) => mod.id));

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

const toModList = (array: ModInfo[]): ModItem[] => {
  return array.map(
    (item) =>
      ({
        id: item.id,
        enabled: item.enabled,
        priority: item.priority,
      }) as const satisfies ModItem,
  );
};
