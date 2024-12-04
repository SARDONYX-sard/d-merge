import { Button, Tooltip } from '@mui/material';

import { useTranslation } from '@/components/hooks/useTranslation';
import type { ModInfo } from '@/services/api/patch';
import { start } from '@/services/api/shell';

import type { GridColDef } from '@mui/x-data-grid';
import type { MouseEventHandler } from 'react';

export const useColumns = () => {
  const { t } = useTranslation();

  const columns = [
    { field: 'id', headerName: 'ID', width: 100, flex: 0.4 },
    {
      field: 'name',
      headerName: t('patch-column-name'),
      flex: 1.2,
    },
    { field: 'author', headerName: t('patch-column-author'), flex: 0.4 },
    {
      field: 'site',
      headerAlign: 'center',
      headerName: t('patch-column-site'),
      flex: 1.2,
      renderCell: (params) => {
        const { site } = params.row;
        const handleMappingClick: MouseEventHandler<HTMLButtonElement> = (event) => {
          event.preventDefault();
          start(site);
        };
        return site === '' ? (
          <></>
        ) : (
          <Tooltip enterNextDelay={1200} placement='left-start' title={site}>
            <Button onClick={handleMappingClick} sx={{ fontSize: 'small', textTransform: 'none' }}>
              {site}
            </Button>
          </Tooltip>
        );
      },
    },
    { field: 'auto', headerName: 'Auto', flex: 1 },
    {
      field: 'priority',
      headerName: t('patch-column-priority'),
      filterable: false,
      flex: 0.3,
      align: 'center',
      headerAlign: 'center',
      renderCell: (params) => params.api.getAllRowIds().indexOf(params.id) + 1,
    },
  ] as const satisfies GridColDef<ModInfo>[];

  return columns;
};
