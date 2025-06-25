import { Button, styled } from '@mui/material';
import Box from '@mui/material/Box';
import { type TreeViewBaseItem, type TreeViewItemId, useTreeViewApiRef } from '@mui/x-tree-view';
import { RichTreeView } from '@mui/x-tree-view/RichTreeView';
import {
  TreeItemCheckbox,
  TreeItemContent,
  TreeItemGroupTransition,
  TreeItemIconContainer,
  TreeItemLabel,
  TreeItemRoot,
} from '@mui/x-tree-view/TreeItem';
import { TreeItemIcon } from '@mui/x-tree-view/TreeItemIcon';
import { TreeItemProvider } from '@mui/x-tree-view/TreeItemProvider';
import { type UseTreeItemParameters, useTreeItem } from '@mui/x-tree-view/useTreeItem';
import { type HTMLAttributes, memo, type Ref, type SyntheticEvent, useCallback, useRef } from 'react';

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
    (_event: SyntheticEvent | null, itemIds: string[]) => {
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

  const handleItemSelectionToggle = useCallback(
    (_event: SyntheticEvent | null, itemId: string, isSelected: boolean) => {
      toggledItemRef.current[itemId] = isSelected;
    },
    [],
  );

  const handleSelectedItemsChange = useCallback(
    (_event: SyntheticEvent | null, newSelectedItems: string[]) => {
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
    <Box sx={{ minHeight: 35, minWidth: 50 }}>
      <Button onClick={handleSelectClick}>
        {selectedFiles.length === 0 ? t('convert.button.select_all') : t('convert.button.unselect_all')}
      </Button>
      <Button onClick={handleExpandClick}>
        {expandedItems.length === 0 ? t('convert.button.expand_all') : t('convert.button.collapse_all')}
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

const CustomTreeItemContent = styled(TreeItemContent)(({ theme }) => ({
  padding: theme.spacing(0.5, 1),
}));

interface CustomTreeItemProps
  extends Omit<UseTreeItemParameters, 'rootRef'>,
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
  } = useTreeItem({ id, itemId, children, label, disabled, rootRef: ref });

  const { convertStatuses } = useConvertContext();

  return (
    <TreeItemProvider id={id} itemId={itemId}>
      <TreeItemRoot {...getRootProps(other)}>
        <CustomTreeItemContent {...getContentProps()}>
          <TreeItemIconContainer {...getIconContainerProps()}>
            <TreeItemIcon status={status} />
          </TreeItemIconContainer>
          <TreeItemCheckbox {...getCheckboxProps()} />
          <Box sx={{ flexGrow: 1, display: 'flex', gap: 1 }}>
            {renderStatusIcon(convertStatuses.get(hashDjb2(itemId)) ?? 0)}
            <TreeItemLabel {...getLabelProps()} />
          </Box>
        </CustomTreeItemContent>
        {children && <TreeItemGroupTransition {...getGroupTransitionProps()} />}
      </TreeItemRoot>
    </TreeItemProvider>
  );
});
