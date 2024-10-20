import { Button, type ButtonProps, Tooltip } from '@mui/material';

import type { ReactNode } from 'react';

type Props = {
  buttonName: ReactNode;
  tooltipTitle?: ReactNode;
} & ButtonProps;

export const ButtonWithToolTip = ({ buttonName, sx, tooltipTitle, ...props }: Props) => (
  <Tooltip placement='top' title={tooltipTitle}>
    <Button
      sx={{
        height: '55px',
        ...sx,
      }}
      variant='outlined'
      {...props}
    >
      {buttonName}
    </Button>
  </Tooltip>
);
