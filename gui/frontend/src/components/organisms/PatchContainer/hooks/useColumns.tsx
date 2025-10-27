import { Button, Tooltip } from '@mui/material';
import type { GridColDef } from '@mui/x-data-grid';
import type { MouseEventHandler } from 'react';
import { useTranslation } from '@/components/hooks/useTranslation';
import type { ModInfo } from '@/services/api/patch';
import { openUrl } from '@/services/api/shell';

export const useColumns = () => {
  const { t } = useTranslation();

  const columns = [
    { field: 'id', headerName: 'ID', flex: 0.4 },
    {
      field: 'name',
      headerName: t('patch.columns.mod_name'),
      flex: 1.2,
    },
    { field: 'mod_type', headerName: t('patch.columns.mod_type'), flex: 0.4 },
    { field: 'author', headerName: t('patch.columns.author'), flex: 0.4 },
    {
      field: 'site',
      headerAlign: 'center',
      headerName: t('patch.columns.site'),
      flex: 1.2,
      renderCell: (params) => {
        const { site } = params.row;
        const handleMappingClick: MouseEventHandler<HTMLButtonElement> = (event) => {
          event.preventDefault();
          openUrl(site);
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
    {
      field: 'priority',
      headerName: t('patch.columns.priority'),
      filterable: false,
      flex: 0.3,
      align: 'center',
      headerAlign: 'center',
    },
  ] as const satisfies GridColDef<ModInfo>[];

  return columns;
};
