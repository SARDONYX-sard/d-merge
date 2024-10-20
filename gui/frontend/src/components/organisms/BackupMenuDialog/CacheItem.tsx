import { Checkbox, Divider, ListItem, ListItemButton, ListItemIcon, ListItemText } from '@mui/material';

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
    <ListItem disablePadding={true} key={title}>
      <ListItemButton dense={true} onClick={onToggle} selected={selected}>
        <ListItemIcon>
          <Checkbox
            checked={selected}
            disableRipple={true}
            edge='start'
            inputProps={{ 'aria-labelledby': labelId }}
            tabIndex={-1}
          />
        </ListItemIcon>
        <ListItemText id={labelId} primary={title} secondary={value} />
      </ListItemButton>
      <Divider />
    </ListItem>
  );
};
