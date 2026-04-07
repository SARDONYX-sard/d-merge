// Forked: https://codesandbox.io/p/sandbox/mui-datagrid-dnd-kit-ctqzj8?file=%2Fsrc%2FApp.tsx%3A1%2C1-71%2C1&from-embed
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { GridRow, type GridRowProps } from '@mui/x-data-grid';
import { type CSSProperties, memo } from 'react';

/**
 * Factory function that creates a draggable (or non-draggable) GridRow component.
 *
 * @param draggable - If true, rows can be reordered by drag & drop.
 *                    If false, dragging is disabled for the entire table.
 * @returns A GridRow component with drag-and-drop support controlled by the flag.
 */
export const createDraggableGridRow = (draggable: boolean = true) => {
  return memo((props: GridRowProps) => {
    const { overIndex, activeIndex, attributes, isDragging, listeners, setNodeRef, transform, transition } =
      useSortable({
        id: props.rowId,
        disabled: !draggable, // Disable all drag interactions when false
      });

    const isSelected = activeIndex === props.index || overIndex === props.index;

    const style: CSSProperties = {
      cursor: !draggable ? 'default' : isDragging ? 'grabbing' : 'grab',
      transform: CSS.Transform.toString(transform),
      transition,
    };

    const id = isSelected ? 'x-data-grid-selected' : undefined;

    return (
      <div
        ref={setNodeRef}
        style={style}
        {...attributes}
        {...(draggable ? listeners : {})} // Attach drag listeners only if enabled
        id={id}
      >
        <GridRow {...props} />
      </div>
    );
  });
};
