import { Button, styled } from '@mui/material';
import Box from '@mui/material/Box';
import { type TreeViewBaseItem, type TreeViewItemId, useTreeViewApiRef } from '@mui/x-tree-view';
import { RichTreeView } from '@mui/x-tree-view/RichTreeView';
import {
  TreeItem2Checkbox,
  TreeItem2Content,
  TreeItem2GroupTransition,
  TreeItem2IconContainer,
  TreeItem2Label,
  TreeItem2Root,
} from '@mui/x-tree-view/TreeItem2';
import { TreeItem2Icon } from '@mui/x-tree-view/TreeItem2Icon';
import { TreeItem2Provider } from '@mui/x-tree-view/TreeItem2Provider';
import { type UseTreeItem2Parameters, useTreeItem2 } from '@mui/x-tree-view/useTreeItem2';
import { type HTMLAttributes, type Ref, type SyntheticEvent, memo, useCallback, useRef } from 'react';

import { useTranslation } from '@/components/hooks/useTranslation';
import { hashDjb2 } from '@/lib/hash-djb2';
import { OBJECT } from '@/lib/object-utils';

import { useConvertContext } from './ConvertProvider';
import { renderStatusIcon } from './renderStatusIcon';

/** Enumerates the selected files in the TreeView. */
export const getAllLeafItemIds = (selectedItems: string[], items: TreeViewBaseItem[]): TreeViewItemId[] => {
  const ids: TreeViewItemId[] = [];

  const registerLeafId = (item: TreeViewBaseItem) => {
    if (!item.children || item.children.length === 0) {
      if (selectedItems.includes(item.id)) {
        ids.push(item.id);
      }
    } else {
      item.children.forEach(registerLeafId);
    }
  };

  for (const item of items) {
    registerLeafId(item);
  }

  return ids;
};

const getItemDescendantsIds = (item: TreeViewBaseItem) => {
  const ids: string[] = [];
  // biome-ignore lint/complexity/noForEach: <explanation>
  item.children?.forEach((child) => {
    ids.push(child.id);
    ids.push(...getItemDescendantsIds(child));
  });
  return ids;
};

/** https://mui.com/x/react-tree-view/rich-tree-view/selection/#controlled-selection */
const getAllItemItemIds = (items: TreeViewBaseItem[]) => {
  const ids: TreeViewItemId[] = [];
  const registerItemId = (item: TreeViewBaseItem) => {
    ids.push(item.id);
    item.children?.forEach(registerItemId);
  };
  items.forEach(registerItemId);

  return ids;
};

/**
 * https://mui.com/x/react-tree-view/rich-tree-view/customization/#custom-icons
 */
export const PathTreeSelector = memo(function PathTreeSelector() {
  const { selectedTree, setSelectedTree } = useConvertContext();
  const toggledItemRef = useRef<{ [itemId: string]: boolean }>({});
  const apiRef = useTreeViewApiRef();
  const { t } = useTranslation();

  //////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
  const [expandedItems, setExpandedItems] = [
    selectedTree.expandedItems,
    (expandedItems: string[]) => {
      setSelectedTree({
        ...selectedTree,
        expandedItems,
      });
    },
  ];

  const handleExpandedItemsChange = useCallback(
    (_event: SyntheticEvent, itemIds: string[]) => {
      setExpandedItems(itemIds);
    },
    [setExpandedItems],
  );

  const handleExpandClick = useCallback(() => {
    setExpandedItems(expandedItems.length === 0 ? getAllItemItemIds(selectedTree.tree) : []);
  }, [expandedItems.length, selectedTree.tree, setExpandedItems]);

  //////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
  const [selectedFiles, setSelectedFiles] = [
    selectedTree.selectedItems,
    (selectedItems: string[]) => {
      setSelectedTree({
        ...selectedTree,
        selectedItems,
      });
    },
  ];

  const handleItemSelectionToggle = useCallback((_event: SyntheticEvent, itemId: string, isSelected: boolean) => {
    toggledItemRef.current[itemId] = isSelected;
  }, []);

  const handleSelectedItemsChange = useCallback(
    (_event: SyntheticEvent, newSelectedItems: string[]) => {
      setSelectedFiles(newSelectedItems);

      // Select / unselect the children of the toggled item
      const itemsToSelect: string[] = [];
      const itemsToUnSelect: { [itemId: string]: boolean } = {};

      for (const [itemId, isSelected] of OBJECT.entries(toggledItemRef.current)) {
        const item = apiRef.current?.getItem(`${itemId}`);
        if (isSelected) {
          itemsToSelect.push(...getItemDescendantsIds(item));
        } else {
          for (const descendantId of getItemDescendantsIds(item)) {
            itemsToUnSelect[descendantId] = true;
          }
        }
      }

      const newSelectedItemsWithChildren = Array.from(
        new Set([...newSelectedItems, ...itemsToSelect].filter((itemId) => !itemsToUnSelect[itemId])),
      );

      setSelectedFiles(newSelectedItemsWithChildren);

      toggledItemRef.current = {};
    },
    [apiRef, setSelectedFiles],
  );

  const handleSelectClick = useCallback(() => {
    setSelectedFiles(selectedFiles.length === 0 ? getAllItemItemIds(selectedTree.tree) : []);
  }, [selectedFiles.length, selectedTree.tree, setSelectedFiles]);
  //////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

  return (
    <Box sx={{ minHeight: 352, minWidth: 250 }}>
      <Button onClick={handleSelectClick}>
        {selectedFiles.length === 0 ? t('convert-select-all') : t('convert-unselect-all')}
      </Button>
      <Button onClick={handleExpandClick}>
        {expandedItems.length === 0 ? t('convert-expand-all') : t('convert-collapse-all')}
      </Button>

      <RichTreeView
        apiRef={apiRef}
        checkboxSelection={true}
        expandedItems={expandedItems}
        items={selectedTree.tree}
        multiSelect={true}
        onExpandedItemsChange={handleExpandedItemsChange}
        onItemSelectionToggle={handleItemSelectionToggle}
        onSelectedItemsChange={handleSelectedItemsChange}
        selectedItems={selectedFiles}
        slots={{ item: CustomTreeItem }}
      />
    </Box>
  );
});

const CustomTreeItemContent = styled(TreeItem2Content)(({ theme }) => ({
  padding: theme.spacing(0.5, 1),
}));

interface CustomTreeItemProps
  extends Omit<UseTreeItem2Parameters, 'rootRef'>,
    Omit<HTMLAttributes<HTMLLIElement>, 'onFocus'> {}

const CustomTreeItem = memo(function CustomTreeItem(props: CustomTreeItemProps, ref?: Ref<HTMLLIElement>) {
  const { id, itemId, label, disabled, children, ...other } = props;

  const {
    getRootProps,
    getContentProps,
    getIconContainerProps,
    getCheckboxProps,
    getLabelProps,
    getGroupTransitionProps,
    status,
  } = useTreeItem2({ id, itemId, children, label, disabled, rootRef: ref });

  const { convertStatuses } = useConvertContext();

  return (
    <TreeItem2Provider itemId={itemId}>
      <TreeItem2Root {...getRootProps(other)}>
        <CustomTreeItemContent {...getContentProps()}>
          <TreeItem2IconContainer {...getIconContainerProps()}>
            <TreeItem2Icon status={status} />
          </TreeItem2IconContainer>
          <TreeItem2Checkbox {...getCheckboxProps()} />
          <Box sx={{ flexGrow: 1, display: 'flex', gap: 1 }}>
            {renderStatusIcon(convertStatuses.get(hashDjb2(itemId)) ?? 0)}
            <TreeItem2Label {...getLabelProps()} />
          </Box>
        </CustomTreeItemContent>
        {children && <TreeItem2GroupTransition {...getGroupTransitionProps()} />}
      </TreeItem2Root>
    </TreeItem2Provider>
  );
});
