import { Box, Checkbox, Divider, ListItem, ListItemButton, ListItemIcon, Typography } from '@mui/material';

import type { ComponentPropsWithRef, ReactNode } from 'react';

type Props = {
  title: string;
  value?: ReactNode;
  selected: boolean;
  onToggle: ComponentPropsWithRef<typeof ListItemButton>['onClick'];
};

export const CacheItem = ({ title, value, selected, onToggle }: Props) => {
  const labelId = `checkbox-list-label-${title}`;
  return (
    <>
      <ListItem disablePadding={true} key={title}>
        <ListItemButton dense={true} onClick={onToggle} selected={selected}>
          <ListItemIcon sx={{ minWidth: 40 }}>
            <Checkbox
              checked={selected}
              disableRipple={true}
              edge='start'
              slotProps={{
                input: {
                  'aria-labelledby': labelId,
                },
              }}
              // tabIndex={-1}
            />
          </ListItemIcon>
        </ListItemButton>

        <Box sx={{ display: 'flex', flexDirection: 'column', flexGrow: 1 }}>
          <Typography color='textSecondary' id={labelId}>
            {title}
          </Typography>
          <Box sx={{ mt: 1 }}>{value}</Box>
          {/*  Also supports cases where value contains <pre> tags */}{' '}
        </Box>
      </ListItem>
      <Divider sx={{ borderColor: '#616161' }} />
    </>
  );
};
