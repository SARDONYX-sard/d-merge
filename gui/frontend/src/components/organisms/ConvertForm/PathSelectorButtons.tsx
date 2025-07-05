import { Grid } from '@mui/material';
import { open } from '@tauri-apps/plugin-dialog';

import { Button } from '@/components/molecules/Button';
import { loadDirNode } from '@/services/api/serde_hkx';

import { useConvertContext } from './ConvertProvider';
import { OutFormatList } from './OutFormatList';
import { SelectionTypeRadios } from './SelectionTypeRadios';

export const PathSelectorButtons = () => {
  const {
    selectionType,

    selectedFiles,
    setSelectedFiles,

    selectedDirs,
    setSelectedDirs,

    treeDirInput,
    setTreeDirInput,
    selectedTree,
    setSelectedTree,

    setConvertStatuses,
  } = useConvertContext();
  const isDirMode = selectionType === 'dir';
  const defaultPath = (() => {
    switch (selectionType) {
      case 'dir':
        return selectedDirs.at(0);
        break;
      case 'files':
        return selectedFiles.at(0);
      case 'tree':
        return treeDirInput;
      default:
        break;
    }
  })();

  const setSelectedPaths = isDirMode ? setSelectedDirs : setSelectedFiles;

  const handlePathSelect = async () => {
    const newSelectedPaths = await open({
      title: isDirMode ? 'Select directory' : 'Select files',
      filters: [{ name: '', extensions: ['hkx', 'xml', 'json', 'yaml'] }],
      multiple: true,
      directory: ['dir', 'tree'].includes(selectionType),
      defaultPath,
    });

    if (selectionType === 'tree') {
      if (newSelectedPaths !== null) {
        setTreeDirInput(newSelectedPaths[0]);
      }

      const roots = (() => {
        if (Array.isArray(newSelectedPaths)) {
          return newSelectedPaths;
        }
        if (newSelectedPaths !== null) {
          return [newSelectedPaths];
        }
      })();

      if (roots) {
        setSelectedTree({ ...selectedTree, roots, tree: await loadDirNode(roots) });
      }
      return;
    }

    if (Array.isArray(newSelectedPaths)) {
      setSelectedPaths(newSelectedPaths);
      setConvertStatuses(new Map()); // Clear the conversion status when a new selection is made.
    } else if (newSelectedPaths !== null) {
      setSelectedPaths([newSelectedPaths]);
      setConvertStatuses(new Map()); // Clear the conversion status when a new selection is made.
    }
  };

  return (
    <Grid container={true} spacing={2} sx={{ justifyContent: 'space-between' }}>
      <Grid>
        <SelectionTypeRadios />
        <Button onClick={handlePathSelect} sx={{ height: '50px', width: '115px' }} />
      </Grid>

      <Grid>
        <OutFormatList />
      </Grid>
    </Grid>
  );
};
