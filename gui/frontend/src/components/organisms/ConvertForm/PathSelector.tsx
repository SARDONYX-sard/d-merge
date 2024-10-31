import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import ErrorIcon from '@mui/icons-material/Error';
import { Box, Chip, Grid2 as Grid } from '@mui/material';
import CircularProgress from '@mui/material/CircularProgress';
import { open } from '@tauri-apps/plugin-dialog';

import { Button } from '@/components/molecules/Button';
import { hashDjb2 } from '@/lib/hash-djb2';

import { useConvertContext } from './ConvertProvider';
import { OutFormatList } from './OutFormatList';
import { SelectionTypeRadios } from './SelectionTypeRadios';

import type { ComponentPropsWithRef } from 'react';

export const PathSelector = () => {
  const {
    selectionType,
    selectedFiles,
    setSelectedFiles,
    selectedDirs,
    setSelectedDirs,
    convertStatuses,
    setConvertStatuses,
  } = useConvertContext();
  const isDirMode = selectionType === 'dir';
  const selectedPaths = isDirMode ? selectedDirs : selectedFiles;
  const setSelectedPaths = isDirMode ? setSelectedDirs : setSelectedFiles;

  const handleDelete: ComponentPropsWithRef<typeof Chip>['onDelete'] = (fileToDelete: string) =>
    setSelectedPaths(selectedPaths.filter((file) => file !== fileToDelete));

  return (
    <Box>
      <Grid container={true} spacing={2}>
        <Grid>
          <Button
            onClick={async () => {
              const newSelectedPaths = await open({
                title: isDirMode ? 'Select directory' : 'Select files',
                filters: [{ name: '', extensions: ['hkx', 'xml', 'json', 'yaml'] }],
                multiple: true,
                directory: isDirMode,
                defaultPath: selectedPaths.at(0),
              });

              if (Array.isArray(newSelectedPaths)) {
                setSelectedPaths(newSelectedPaths);
                setConvertStatuses(new Map()); // Clear the conversion status when a new selection is made.
              } else if (newSelectedPaths !== null) {
                setSelectedPaths([newSelectedPaths]);
                setConvertStatuses(new Map()); // Clear the conversion status when a new selection is made.
              }
            }}
          />
        </Grid>
        <Grid>
          <SelectionTypeRadios />
        </Grid>

        <Grid>
          <OutFormatList />
        </Grid>
      </Grid>

      <Box mt={2}>
        {selectedPaths.map((path) => {
          const pathId = hashDjb2(path);
          const statusId = convertStatuses.get(pathId) ?? 0;

          return (
            <Chip icon={renderStatusIcon(statusId)} key={pathId} label={path} onDelete={() => handleDelete(path)} />
          );
        })}
      </Box>
    </Box>
  );
};

const renderStatusIcon = (status: number) => {
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
