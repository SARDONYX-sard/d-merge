import { arrayMove } from '@dnd-kit/sortable';
import { useCallback, useEffect, useState } from 'react';

import { NOTIFY } from '@/lib/notify';
import { type ModInfo, loadActivateMods, loadModsInfo } from '@/services/api/patch';

import type { Props as DndCtxProps } from '@dnd-kit/core/dist/components/DndContext/DndContext';
import type { DataGridPropsWithoutDefaultValue } from '@mui/x-data-grid/internals';

type DragEndHandler = Exclude<DndCtxProps['onDragEnd'], undefined>;

type Props = {
  loadModsInfoFn: () => Promise<ModInfo[]>;
  loadActivateModsFn: () => Promise<readonly string[]>;
};

const defaultProps = {
  loadModsInfoFn: loadModsInfo,
  loadActivateModsFn: loadActivateMods,
};

/**
 * # Mocks
 * ```ts
 * import { createMockModsInfo, createMockSelectId, type ModInfo } from '@/services/api/patch'; // Mock
 * const modsInfoFn = createMockModsInfo; // Mock
 * const activateModsFn = async () => createMockSelectId(modsInfo); // Mock
 * useModsInfo({ modsInfoFn, activateModsFn })
 * ```
 */
export function useModsInfo({ loadActivateModsFn, loadModsInfoFn }: Props = defaultProps) {
  const [rows, setRows] = useState<ModInfo[]>([]);
  const [selectionModel, setSelectionModel] = useState<readonly string[]>([]);

  useEffect(() => {
    NOTIFY.asyncTry(async () => {
      const modsInfo = await loadModsInfoFn();
      const activateMods = await loadActivateModsFn();
      setRows(modsInfo);
      setSelectionModel(activateMods);
    });
  }, [loadActivateModsFn, loadModsInfoFn]);

  const handleDragEnd = useCallback<DragEndHandler>(
    ({ active, over }) => {
      if (over) {
        const oldIndex = rows.findIndex((row) => row.id === active.id);
        const newIndex = rows.findIndex((row) => row.id === over.id);
        setRows((prevRows) => arrayMove(prevRows, oldIndex, newIndex));
      }
    },
    [rows],
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

  return {
    rows,
    /** IDs
     * @example ['aaa', 'bbb']
     */
    selectionModel,
    handleDragEnd,
    handleRowSelectionModelChange,
  };
}
