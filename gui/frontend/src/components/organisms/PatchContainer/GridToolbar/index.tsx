import CancelIcon from '@mui/icons-material/Cancel';
import FileDownloadIcon from '@mui/icons-material/FileDownload';
import FilterListIcon from '@mui/icons-material/FilterList';
import SearchIcon from '@mui/icons-material/Search';
import ViewColumnIcon from '@mui/icons-material/ViewColumn';
import Badge from '@mui/material/Badge';
import Divider from '@mui/material/Divider';
import InputAdornment from '@mui/material/InputAdornment';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import { styled } from '@mui/material/styles';
import TextField from '@mui/material/TextField';
import Tooltip from '@mui/material/Tooltip';
import {
  ColumnsPanelTrigger,
  ExportCsv,
  ExportPrint,
  FilterPanelTrigger,
  QuickFilter,
  QuickFilterClear,
  QuickFilterControl,
  QuickFilterTrigger,
  Toolbar,
  ToolbarButton,
  useGridApiContext,
} from '@mui/x-data-grid';
import React from 'react';

import { CustomDensitySelector } from './CustomDencity';
import { useSortClearButton } from './useSortClearButton';

type OwnerState = {
  expanded: boolean;
};

const StyledQuickFilter = styled(QuickFilter)({
  display: 'grid',
  alignItems: 'center',
});

const StyledToolbarButton = styled(ToolbarButton)<{ ownerState: OwnerState }>(({ theme, ownerState }) => ({
  gridArea: '1 / 1',
  width: 'min-content',
  height: 'min-content',
  zIndex: 1,
  opacity: ownerState.expanded ? 0 : 1,
  pointerEvents: ownerState.expanded ? 'none' : 'auto',
  transition: theme.transitions.create(['opacity']),
}));

const StyledTextField = styled(TextField)<{
  ownerState: OwnerState;
}>(({ theme, ownerState }) => ({
  gridArea: '1 / 1',
  overflowX: 'clip',
  width: ownerState.expanded ? 260 : 'var(--trigger-width)',
  opacity: ownerState.expanded ? 1 : 0,
  transition: theme.transitions.create(['width', 'opacity']),
}));

export const CustomToolbar = () => {
  const apiRef = useGridApiContext();
  const [exportMenuOpen, setExportMenuOpen] = React.useState(false);
  const exportMenuTriggerRef = React.useRef<HTMLButtonElement>(null);
  const SortClearButton = useSortClearButton();

  return (
    <Toolbar>
      {SortClearButton}

      <Tooltip title={apiRef.current.getLocaleText('toolbarColumns')}>
        <ColumnsPanelTrigger render={<ToolbarButton />}>
          <ViewColumnIcon fontSize='small' />
        </ColumnsPanelTrigger>
      </Tooltip>

      <Tooltip title={apiRef.current.getLocaleText('toolbarFilters')}>
        <FilterPanelTrigger
          render={(props, state) => (
            <ToolbarButton {...props} color='default'>
              <Badge badgeContent={state.filterCount} color='primary' variant='dot'>
                <FilterListIcon fontSize='small' />
              </Badge>
            </ToolbarButton>
          )}
        />
      </Tooltip>

      {/* Density */}
      <Tooltip title={apiRef.current.getLocaleText('toolbarDensity')}>
        <CustomDensitySelector />
      </Tooltip>

      <Divider flexItem={true} orientation='vertical' sx={{ mx: 0.5 }} variant='middle' />

      <Tooltip title={apiRef.current.getLocaleText('toolbarExport')}>
        <ToolbarButton
          aria-controls='export-menu'
          aria-expanded={exportMenuOpen ? 'true' : undefined}
          aria-haspopup='true'
          id='export-menu-trigger'
          onClick={() => setExportMenuOpen(true)}
          ref={exportMenuTriggerRef}
        >
          <FileDownloadIcon fontSize='small' />
        </ToolbarButton>
      </Tooltip>

      <Menu
        anchorEl={exportMenuTriggerRef.current}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
        id='export-menu'
        onClose={() => setExportMenuOpen(false)}
        open={exportMenuOpen}
        slotProps={{
          list: {
            'aria-labelledby': 'export-menu-trigger',
          },
        }}
        transformOrigin={{ vertical: 'top', horizontal: 'right' }}
      >
        <ExportPrint onClick={() => setExportMenuOpen(false)} render={<MenuItem />}>
          {apiRef.current.getLocaleText('toolbarExportPrint')}
        </ExportPrint>
        <ExportCsv onClick={() => setExportMenuOpen(false)} render={<MenuItem />}>
          {apiRef.current.getLocaleText('toolbarExportCSV')}
        </ExportCsv>
      </Menu>

      <Divider flexItem={true} orientation='vertical' sx={{ mx: 0.5 }} variant='middle' />

      <StyledQuickFilter>
        <QuickFilterTrigger
          render={(triggerProps, state) => (
            <Tooltip enterDelay={0} title={apiRef.current.getLocaleText('toolbarQuickFilterLabel')}>
              <StyledToolbarButton
                {...triggerProps}
                aria-disabled={state.expanded}
                color='default'
                ownerState={{ expanded: state.expanded }}
              >
                <SearchIcon fontSize='small' />
              </StyledToolbarButton>
            </Tooltip>
          )}
        />
        <QuickFilterControl
          render={({ ref, ...controlProps }, state) => (
            <StyledTextField
              {...controlProps}
              aria-label={apiRef.current.getLocaleText('toolbarQuickFilterLabel')}
              inputRef={ref}
              ownerState={{ expanded: state.expanded }}
              placeholder={apiRef.current.getLocaleText('toolbarQuickFilterPlaceholder')}
              size='small'
              slotProps={{
                input: {
                  startAdornment: (
                    <InputAdornment position='start'>
                      <SearchIcon fontSize='small' />
                    </InputAdornment>
                  ),
                  endAdornment: state.value ? (
                    <InputAdornment position='end'>
                      <QuickFilterClear
                        aria-label={apiRef.current.getLocaleText('toolbarQuickFilterDeleteIconLabel')}
                        edge='end'
                        material={{ sx: { marginRight: -0.75 } }}
                        size='small'
                      >
                        <CancelIcon fontSize='small' />
                      </QuickFilterClear>
                    </InputAdornment>
                  ) : null,
                  ...controlProps.slotProps?.input,
                },
                ...controlProps.slotProps,
              }}
            />
          )}
        />
      </StyledQuickFilter>
    </Toolbar>
  );
};
