import CloseIcon from '@mui/icons-material/Close';
import { Box, Tab, Tabs } from '@mui/material';
import React, { Dispatch, SetStateAction } from 'react';
import { FileTab } from './HkannoTabEditor';

type Props = {
  tabs: FileTab[];
  active: number;
  setActive: (index: number) => void;
  setTabs: Dispatch<SetStateAction<FileTab[]>>;
};

export const ClosableTabs: React.FC<Props> = ({ tabs, active, setActive, setTabs }) => {
  const handleClose = (id: string, index: number) => {
    setTabs((prev) => prev.filter((fileTab) => fileTab.id !== id));
    if (active >= index && active > 0) {
      setActive(active - 1);
    }
  };

  return (
    <Tabs
      value={active}
      onChange={(_, v) => setActive(v)}
      variant='scrollable'
      scrollButtons='auto'
      textColor='inherit'
    >
      {tabs.map((tab, index) => (
        <Tab
          key={tab.id}
          sx={{ textTransform: 'none' }}
          label={
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
              <span>{tab.inputPath.split(/[\\/]/).pop()}</span>
              <Box
                component='span'
                sx={{
                  ml: 0.5,
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  width: 18,
                  height: 18,
                  borderRadius: '50%',
                  '&:hover': { backgroundColor: 'rgba(255,255,255,0.1)' },
                  cursor: 'pointer',
                }}
                onClick={(e) => {
                  e.stopPropagation();
                  handleClose(tab.id, index);
                }}
              >
                <CloseIcon sx={{ fontSize: 14 }} />
              </Box>
            </Box>
          }
        />
      ))}
    </Tabs>
  );
};
