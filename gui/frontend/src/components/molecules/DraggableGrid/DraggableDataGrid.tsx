// Forked: https://codesandbox.io/p/sandbox/mui-datagrid-dnd-kit-ctqzj8?file=%2Fsrc%2FApp.tsx%3A1%2C1-71%2C1&from-embed
import { DndContext, MouseSensor, type UniqueIdentifier, closestCorners, useSensor, useSensors } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import { DataGrid, type DataGridProps } from '@mui/x-data-grid';
import { memo } from 'react';

import { DraggableGridRow } from './DraggableGridRow';

import type { Props as DndCtxProps } from '@dnd-kit/core/dist/components/DndContext/DndContext';

type Id =
  | UniqueIdentifier
  | {
      id: UniqueIdentifier;
    };
type Props = DataGridProps & {
  rows: Id[];
  onDragEnd: DndCtxProps['onDragEnd'];
};

export const DraggableDataGrid = memo(function DraggableGrid({ rows, onDragEnd, ...props }: Props) {
  const sensors = useSensors(
    useSensor(MouseSensor, {
      activationConstraint: {
        // 5px Enable sorting functionality when dragging. See: https://www.gaji.jp/blog/2022/03/10/9281/
        // Why need this?: If a button is in a draggable cell, the dragging is given priority and the button is prevented from being pressed.
        distance: 5,
      },
    }),
  );

  return (
    <div>
      <DndContext autoScroll={{
        enabled: true,
        /**
         * NOTE: Set to false to avoid the scroll position force-return bug.
         * ref: https://github.com/clauderic/dnd-kit/issues/825#issuecomment-1459030786
         */
        layoutShiftCompensation: false,
        threshold: { x: 0, y: 0.2 } // Eliminate horizontal auto-scroll
      }} collisionDetection={closestCorners} onDragEnd={onDragEnd} sensors={sensors}
      >
        <SortableContext items={rows} strategy={verticalListSortingStrategy}>
          <DataGrid
            checkboxSelection={true}
            disableColumnSorting={true} // Because they cannot be reordered when reordering is applied: https://github.com/mui/mui-x/issues/10706
            disableRowSelectionOnClick={true}
            rowBufferPx={2000} // Without this, rows appear to disappear when auto-scroll is used to drag rows out of range.
            rows={rows}
            showCellVerticalBorder={true}
            slots={{ row: DraggableGridRow }}
            {...props}
          />
        </SortableContext>
      </DndContext>
    </div>
  );
});
