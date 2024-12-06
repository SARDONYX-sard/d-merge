import { Box, Chip } from '@mui/material';

import { hashDjb2 } from '@/lib/hash-djb2';

import { useConvertContext } from './ConvertProvider';
import { PathTreeSelector } from './PathTreeSelector';
import { renderStatusIcon } from './renderStatusIcon';

import type { ComponentPropsWithRef } from 'react';

export const PathSelector = () => {
  const { selectionType, selectedFiles, setSelectedFiles, selectedDirs, setSelectedDirs, convertStatuses } =
    useConvertContext();
  const isDirMode = selectionType === 'dir';
  const selectedPaths = isDirMode ? selectedDirs : selectedFiles;
  const setSelectedPaths = isDirMode ? setSelectedDirs : setSelectedFiles;

  const handleDelete: ComponentPropsWithRef<typeof Chip>['onDelete'] = (fileToDelete: string) =>
    setSelectedPaths(selectedPaths.filter((file) => file !== fileToDelete));

  return (
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
  );
};
