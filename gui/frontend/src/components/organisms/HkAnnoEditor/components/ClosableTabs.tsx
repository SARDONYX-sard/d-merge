import CloseIcon from '@mui/icons-material/Close';
import { Box, Tab, Tabs, Tooltip } from '@mui/material';
import { useEditorContext } from '../context/editorContext';

export const ClosableTabs = () => {
  const [state, dispatch] = useEditorContext();

  return (
    <Tabs value={state.active} onChange={(_, v) => dispatch({ type: 'SET_ACTIVE', index: v })} variant='scrollable'>
      {state.tabs.map((tab, i) => (
        <Tooltip title={tab.id} enterDelay={2000}>
          <Tab
            key={tab.id}
            label={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, textTransform: 'none' }}>
                {tab.inputPath.split(/[\\/]/).pop()}
                <CloseIcon
                  sx={{ fontSize: 14 }}
                  onClick={(e) => {
                    e.stopPropagation();
                    dispatch({ type: 'CLOSE_TAB', index: i });
                  }}
                />
              </Box>
            }
          />
        </Tooltip>
      ))}
    </Tabs>
  );
};
