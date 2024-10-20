// Forked: https://codesandbox.io/p/sandbox/mui-datagrid-dnd-kit-ctqzj8?file=%2Fsrc%2FApp.tsx%3A1%2C1-71%2C1&from-embed
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { GridRow, type GridRowProps } from '@mui/x-data-grid';
import { type CSSProperties, memo } from 'react';

export const DraggableGridRow = memo(function DraggableGrid(params: GridRowProps) {
  const { activeIndex, attributes, isDragging, listeners, setNodeRef, transform, transition } = useSortable({
    id: params.rowId,
  });

  const isSelected = activeIndex === params.index;
  const style: CSSProperties = {
    cursor: isDragging ? 'grabbing' : 'grab',
    transform: CSS.Transform.toString(transform),
    transition,
    backgroundColor: isSelected ? '#404755' : undefined,
  };
  const id = isSelected ? 'x-data-grid-selected' : undefined;

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners} id={id}>
      <GridRow {...params} />
    </div>
  );
});
