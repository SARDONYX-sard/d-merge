'use client';
import TabContext from '@mui/lab/TabContext';
import TabList from '@mui/lab/TabList';
import TabPanel from '@mui/lab/TabPanel';
import { Box } from '@mui/material';
import Tab from '@mui/material/Tab';

import { useStorageState } from '@/components/hooks/useStorageState/useStorageState';
import { useTranslation } from '@/components/hooks/useTranslation';
import { ImportLangButton } from '@/components/molecules/ImportLangButton';
import { JsAutoRunButton } from '@/components/molecules/JsAutoRunButton';
import { BackupExportButton } from '@/components/organisms/BackupExportButton';
import { BackupImportButton } from '@/components/organisms/BackupImportButton';
import { CssList } from '@/components/organisms/CssList';
import { EditorList } from '@/components/organisms/EditorList/EditorList';
import { I18nList } from '@/components/organisms/I18nList';
import { NotifyList } from '@/components/organisms/NotifyList';

import type { SyntheticEvent } from 'react';

export const Tabs = () => {
  const [selectedTab, setSelectedTab] = useStorageState('settings-tab-select', 'editor');
  const { t } = useTranslation();

  const handleChange = (_event: SyntheticEvent, tabId: string) => setSelectedTab(tabId);

  return (
    <Box
      sx={{
        minWidth: 'max-content',
        typography: 'body1',
      }}
    >
      <TabContext value={selectedTab}>
        <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
          <TabList aria-label='Setting tabs' onChange={handleChange}>
            <Tab label={t('tab-label-editor')} value='editor' />
            <Tab label={t('tab-label-notice')} value='notice' />
            <Tab label={t('tab-label-lang')} value='lang' />
            <Tab label={t('tab-label-backup')} value='backup' />
          </TabList>
        </Box>

        <TabPanel value='editor'>
          <EditorList />
          <CssList />
          <JsAutoRunButton sx={{ marginLeft: '10px' }} />
        </TabPanel>

        <TabPanel value='notice'>
          <NotifyList />
        </TabPanel>

        <TabPanel value='lang'>
          <ImportLangButton />
          <I18nList />
        </TabPanel>

        <TabPanel value='backup'>
          <BackupImportButton />
          <BackupExportButton />
        </TabPanel>
      </TabContext>
    </Box>
  );
};
