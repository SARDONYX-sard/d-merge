import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import { type ComponentPropsWithRef, type ReactNode, useId } from 'react';

import { Button } from '@/components/molecules/Button';

type Props = {
  label: string;
  icon: ReactNode;
  path: string;
  setPath: (path: string) => void;
} & ComponentPropsWithRef<typeof Button>;

export function InputWithIcon({ label, icon, path, setPath, ...props }: Props) {
  const id = useId();

  return (
    <Box sx={{ '& > :not(style)': { m: 1 } }}>
      <Box sx={{ display: 'flex', alignItems: 'flex-end' }}>
        {icon}
        <TextField
          id={id}
          label={label}
          onChange={({ target }) => setPath(target.value)}
          sx={{ width: '100%', paddingRight: '10px' }}
          value={path}
          variant='standard'
        />
        <Button {...props} />
      </Box>
    </Box>
  );
}
