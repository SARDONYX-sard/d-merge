import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import { type ComponentPropsWithRef, type ReactNode, useId } from 'react';

import { Button } from '@/components/molecules/Button';

type Props = {
  label: string;
  icon: ReactNode;
  endIcon?: ReactNode;
  path: string;
  setPath: (path: string) => void;
  placeholder?: string;
} & ComponentPropsWithRef<typeof Button>;

export function InputField({ label, icon, endIcon, path, setPath, placeholder, disabled, ...props }: Props) {
  const id = useId();

  return (
    <Box sx={{ '& > :not(style)': { m: 1 } }}>
      <Box sx={{ display: 'flex', alignItems: 'flex-end' }}>
        {icon}
        <TextField
          disabled={disabled}
          id={id}
          label={label}
          onChange={({ target }) => setPath(target.value)}
          placeholder={placeholder}
          sx={{ width: '100%', paddingRight: '10px' }}
          value={path}
          variant='standard'
        />
        {endIcon}
        <Button disabled={disabled} {...props} sx={{ height: '50px', width: '125px' }} />
      </Box>
    </Box>
  );
}
