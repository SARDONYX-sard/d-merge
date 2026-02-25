import FormatLineSpacingIcon from '@mui/icons-material/FormatLineSpacing';
import IconButton from '@mui/material/IconButton';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Tooltip from '@mui/material/Tooltip';
import { useGridApiContext } from '@mui/x-data-grid';
import React from 'react';

export function CustomDensitySelector() {
  const apiRef = useGridApiContext();
  const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null);

  const handleOpen = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget);
  };

  const handleClose = () => {
    setAnchorEl(null);
  };

  const handleSelect = (density: 'compact' | 'standard' | 'comfortable') => {
    apiRef.current.setDensity(density);
    handleClose();
  };

  return (
    <>
      <Tooltip title={apiRef.current.getLocaleText('toolbarDensity')}>
        <IconButton onClick={handleOpen} size='small'>
          <FormatLineSpacingIcon fontSize='small' />
        </IconButton>
      </Tooltip>
      <Menu
        anchorEl={anchorEl}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
        onClose={handleClose}
        open={Boolean(anchorEl)}
        transformOrigin={{ vertical: 'top', horizontal: 'right' }}
      >
        <MenuItem onClick={() => handleSelect('compact')}>
          {apiRef.current.getLocaleText('toolbarDensityCompact')}
        </MenuItem>
        <MenuItem onClick={() => handleSelect('standard')}>
          {apiRef.current.getLocaleText('toolbarDensityStandard')}
        </MenuItem>
        <MenuItem onClick={() => handleSelect('comfortable')}>
          {apiRef.current.getLocaleText('toolbarDensityComfortable')}
        </MenuItem>
      </Menu>
    </>
  );
}
