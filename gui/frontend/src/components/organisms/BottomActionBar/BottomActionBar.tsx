import { Box, type SxProps, type Theme } from '@mui/material';
import type { ComponentPropsWithRef, ReactNode } from 'react';
import { ConvertButton } from '@/components/atoms/ConvertButton';
import { LogDirButton } from '@/components/molecules/LogDirButton';
import { LogFileButton } from '@/components/molecules/LogFileButton';
import { LogLevelList } from '@/components/organisms/LogLevelList';

const sx: SxProps<Theme> = {
  position: 'fixed',
  bottom: 50,
  width: '100%',
  display: 'flex',
  alignItems: 'center',
  padding: '10px',
  justifyContent: 'space-between',
  backgroundColor: '#252525d8',
};

type Props = ComponentPropsWithRef<typeof ConvertButton & ReactNode>;

const MenuPadding = () => <div style={{ height: '100px' }} />;
export const BottomActionBar = ({ children, ...others }: Props) => {
  return (
    <>
      <MenuPadding />
      <Box sx={sx}>
        <LogLevelList />
        <LogDirButton />
        <LogFileButton />
        {children}
        <ConvertButton {...others} />
      </Box>
    </>
  );
};
