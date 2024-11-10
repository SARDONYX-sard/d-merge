import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import ErrorIcon from '@mui/icons-material/Error';
import { Box, Chip, Grid2 as Grid } from '@mui/material';
import CircularProgress from '@mui/material/CircularProgress';
import { open } from '@tauri-apps/plugin-dialog';

import { Button } from '@/components/molecules/Button';
import { hashDjb2 } from '@/lib/hash-djb2';
import { loadDirNode } from '@/services/api/serde_hkx';

import { useConvertContext } from './ConvertProvider';
import { OutFormatList } from './OutFormatList';
import { PathTreeSelector } from './PathTreeSelector';
import { SelectionTypeRadios } from './SelectionTypeRadios';

import type { ComponentPropsWithRef } from 'react';

export const PathSelector = () => {
  const {
    selectionType,
    selectedFiles,
    setSelectedFiles,
    selectedDirs,
    setSelectedDirs,
    selectedTree,
    setSelectedTree,
    convertStatuses,
    setConvertStatuses,
  } = useConvertContext();
  const isDirMode = selectionType === 'dir';
  const selectedPaths = isDirMode ? selectedDirs : selectedFiles;
  const setSelectedPaths = isDirMode ? setSelectedDirs : setSelectedFiles;

  const handleDelete: ComponentPropsWithRef<typeof Chip>['onDelete'] = (fileToDelete: string) =>
    setSelectedPaths(selectedPaths.filter((file) => file !== fileToDelete));

  const handlePathSelect = async () => {
    const newSelectedPaths = await open({
      title: isDirMode ? 'Select directory' : 'Select files',
      filters: [{ name: '', extensions: ['hkx', 'xml', 'json', 'yaml'] }],
      multiple: true,
      directory: ['dir', 'tree'].includes(selectionType),
      defaultPath: selectedPaths.at(0),
    });

    if (selectionType === 'tree') {
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
    <Box>
      <Grid container={true} spacing={2} sx={{ justifyContent: 'space-between' }}>
        <Grid>
          <SelectionTypeRadios />
          <Button onClick={handlePathSelect} />
        </Grid>

        <Grid>
          <OutFormatList />
        </Grid>
      </Grid>

      <Box mt={2}>
        {selectionType === 'tree' ? (
          <PathTreeSelector />
        ) : (
          selectedPaths.map((path) => {
            const pathId = hashDjb2(path);
            const statusId = convertStatuses.get(pathId) ?? 0;

            return (
              <Chip icon={renderStatusIcon(statusId)} key={pathId} label={path} onDelete={() => handleDelete(path)} />
            );
          })
        )}
      </Box>
    </Box>
  );
};

export const renderStatusIcon = (status: number) => {
  switch (status) {
    case 1:
      return <CircularProgress size={20} />;
    case 2:
      return <CheckCircleIcon color='success' />;
    case 3:
      return <ErrorIcon color='error' />;
    default:
      return undefined;
  }
};
